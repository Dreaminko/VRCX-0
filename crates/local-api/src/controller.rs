use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::auth::{generate_local_api_token, LocalApiAuthPolicy};
use crate::config::{
    LocalApiConfigStore, DEFAULT_LOCAL_API_PORT, LOCAL_API_ALLOW_LAN_CONFIG_KEY,
    LOCAL_API_ENABLED_CONFIG_KEY, LOCAL_API_PORT_CONFIG_KEY, LOCAL_API_TOKEN_CONFIG_KEY,
};
use crate::state::RoomState;
use crate::transport::{
    bind_local_api_listener, build_local_api_router, LocalApiRouterState, ServerHub,
};
use crate::types::{LocalApiFailure, LocalApiServerState, LocalApiStatus};
use crate::wire::ByeReason;
use crate::LocalApiError;

pub struct LocalApiController {
    config: Arc<dyn LocalApiConfigStore>,
    app_version: String,
    active_connections: Arc<AtomicU32>,
    next_listener_id: AtomicU64,
    state: Arc<tokio::sync::Mutex<ControllerState>>,
}

struct ControllerState {
    game_running: bool,
    token: String,
    handle: Option<ListenerHandle>,
    hub: Option<Arc<ServerHub>>,
    last_error: Option<LocalApiFailure>,
    session_generation: u64,
}

struct ListenerHandle {
    id: u64,
    port: u16,
    accept_cancel: CancellationToken,
    session_cancel: CancellationToken,
    join: JoinHandle<()>,
}

impl LocalApiController {
    pub fn new(
        config: Arc<dyn LocalApiConfigStore>,
        app_version: String,
    ) -> Result<Self, LocalApiError> {
        let token = ensure_token(config.as_ref())?;
        Ok(Self {
            config,
            app_version,
            active_connections: Arc::new(AtomicU32::new(0)),
            next_listener_id: AtomicU64::new(0),
            state: Arc::new(tokio::sync::Mutex::new(ControllerState {
                game_running: false,
                token,
                handle: None,
                hub: None,
                last_error: None,
                session_generation: 0,
            })),
        })
    }

    pub async fn start_from_config(&self) -> Result<LocalApiStatus, LocalApiError> {
        let mut state = self.state.lock().await;
        if !self.configured_enabled()? || !state.game_running || state.handle.is_some() {
            return self.status_locked(&state);
        }
        if let Err(error) = self.start_new_session(&mut state).await {
            state.last_error = Some(LocalApiFailure::from_error(&error));
            return Err(error);
        }
        self.status_locked(&state)
    }

    pub async fn set_game_running(
        &self,
        game_running: bool,
    ) -> Result<LocalApiStatus, LocalApiError> {
        let mut state = self.state.lock().await;
        if state.game_running == game_running {
            return self.status_locked(&state);
        }
        state.game_running = game_running;
        if game_running {
            if self.configured_enabled()? {
                if let Err(error) = self.start_new_session(&mut state).await {
                    state.last_error = Some(LocalApiFailure::from_error(&error));
                    return Err(error);
                }
            }
        } else {
            if let Some(hub) = state.hub.take() {
                hub.send_bye(ByeReason::GameStopped);
                hub.clear();
            }
            if let Some(handle) = state.handle.take() {
                self.stop_listener(handle, true).await;
            }
            state.session_generation = state.session_generation.saturating_add(1);
            state.last_error = None;
        }
        self.status_locked(&state)
    }

    pub async fn publish(&self, room: Option<RoomState>) {
        let state = self.state.lock().await;
        if state.handle.is_some() {
            if let Some(hub) = &state.hub {
                hub.publish(room);
            }
        }
    }

    pub async fn set_enabled(&self, enabled: bool) -> Result<LocalApiStatus, LocalApiError> {
        let mut state = self.state.lock().await;
        if enabled == self.configured_enabled()? {
            return self.status_locked(&state);
        }
        if enabled {
            if state.game_running {
                let port = self.configured_port()?;
                let allow_lan_connections = self.configured_allow_lan_connections()?;
                let hub = Arc::new(ServerHub::new(self.app_version.clone()));
                let handle = match self
                    .start_listener(port, allow_lan_connections, &state.token, Arc::clone(&hub))
                    .await
                {
                    Ok(handle) => handle,
                    Err(error) => {
                        state.last_error = Some(LocalApiFailure::from_error(&error));
                        return Err(error);
                    }
                };
                if let Err(error) = self.config.set_bool(LOCAL_API_ENABLED_CONFIG_KEY, true) {
                    self.stop_listener(handle, false).await;
                    state.last_error = Some(LocalApiFailure::from_error(&error));
                    return Err(error);
                }
                state.handle = Some(handle);
                state.hub = Some(hub);
                state.session_generation = state.session_generation.saturating_add(1);
            } else {
                self.config.set_bool(LOCAL_API_ENABLED_CONFIG_KEY, true)?;
            }
        } else {
            self.config.set_bool(LOCAL_API_ENABLED_CONFIG_KEY, false)?;
            if let Some(hub) = state.hub.take() {
                hub.send_bye(ByeReason::Disabled);
                hub.clear();
            }
            if let Some(handle) = state.handle.take() {
                self.stop_listener(handle, true).await;
            }
            state.session_generation = state.session_generation.saturating_add(1);
        }
        state.last_error = None;
        self.status_locked(&state)
    }

    pub async fn set_port(&self, port: u16) -> Result<LocalApiStatus, LocalApiError> {
        validate_port(port)?;
        let mut state = self.state.lock().await;
        let previous_port = self.configured_port()?;
        if port == previous_port {
            return self.status_locked(&state);
        }
        let allow_lan_connections = self.configured_allow_lan_connections()?;
        let should_run = self.configured_enabled()? && state.game_running;
        let Some(hub) = state.hub.clone() else {
            if should_run {
                let hub = Arc::new(ServerHub::new(self.app_version.clone()));
                let handle = match self
                    .start_listener(port, allow_lan_connections, &state.token, Arc::clone(&hub))
                    .await
                {
                    Ok(handle) => handle,
                    Err(error) => {
                        state.last_error = Some(LocalApiFailure::from_error(&error));
                        return Err(error);
                    }
                };
                if let Err(error) = self
                    .config
                    .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
                {
                    self.stop_listener(handle, false).await;
                    state.last_error = Some(LocalApiFailure::from_error(&error));
                    return Err(error);
                }
                state.handle = Some(handle);
                state.hub = Some(hub);
                state.session_generation = state.session_generation.saturating_add(1);
                state.last_error = None;
                return self.status_locked(&state);
            }
            drop(bind_local_api_listener(port, allow_lan_connections)?);
            self.config
                .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())?;
            state.last_error = None;
            return self.status_locked(&state);
        };

        let new_handle = match self
            .start_listener(port, allow_lan_connections, &state.token, Arc::clone(&hub))
            .await
        {
            Ok(handle) => handle,
            Err(error) => {
                state.last_error = Some(LocalApiFailure::from_error(&error));
                return Err(error);
            }
        };
        if let Err(error) = self
            .config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
        {
            self.stop_listener(new_handle, false).await;
            state.last_error = Some(LocalApiFailure::from_error(&error));
            return Err(error);
        }
        let previous_handle = state.handle.replace(new_handle);
        state.last_error = None;
        if let Some(previous_handle) = previous_handle {
            self.stop_listener(previous_handle, false).await;
        }
        self.status_locked(&state)
    }

    pub async fn set_allow_lan_connections(
        &self,
        allow_lan_connections: bool,
    ) -> Result<LocalApiStatus, LocalApiError> {
        let mut state = self.state.lock().await;
        let previous_allow_lan_connections = self.configured_allow_lan_connections()?;
        if allow_lan_connections == previous_allow_lan_connections {
            return self.status_locked(&state);
        }
        let port = self.configured_port()?;
        let should_run = self.configured_enabled()? && state.game_running;
        let Some(hub) = state.hub.clone() else {
            if should_run {
                let hub = Arc::new(ServerHub::new(self.app_version.clone()));
                let handle = match self
                    .start_listener(port, allow_lan_connections, &state.token, Arc::clone(&hub))
                    .await
                {
                    Ok(handle) => handle,
                    Err(error) => {
                        state.last_error = Some(LocalApiFailure::from_error(&error));
                        return Err(error);
                    }
                };
                if let Err(error) = self
                    .config
                    .set_bool(LOCAL_API_ALLOW_LAN_CONFIG_KEY, allow_lan_connections)
                {
                    self.stop_listener(handle, false).await;
                    state.last_error = Some(LocalApiFailure::from_error(&error));
                    return Err(error);
                }
                state.handle = Some(handle);
                state.hub = Some(hub);
                state.session_generation = state.session_generation.saturating_add(1);
                state.last_error = None;
                return self.status_locked(&state);
            }
            drop(bind_local_api_listener(port, allow_lan_connections)?);
            self.config
                .set_bool(LOCAL_API_ALLOW_LAN_CONFIG_KEY, allow_lan_connections)?;
            state.last_error = None;
            return self.status_locked(&state);
        };
        let previous_handle = state.handle.take();
        if let Some(previous_handle) = previous_handle {
            self.stop_listener(previous_handle, false).await;
        }
        let new_handle = match self
            .start_listener(port, allow_lan_connections, &state.token, Arc::clone(&hub))
            .await
        {
            Ok(handle) => handle,
            Err(error) => {
                state.handle = self
                    .start_listener(port, previous_allow_lan_connections, &state.token, hub)
                    .await
                    .ok();
                state.last_error = Some(LocalApiFailure::from_error(&error));
                return Err(error);
            }
        };
        if let Err(error) = self
            .config
            .set_bool(LOCAL_API_ALLOW_LAN_CONFIG_KEY, allow_lan_connections)
        {
            self.stop_listener(new_handle, false).await;
            state.handle = self
                .start_listener(port, previous_allow_lan_connections, &state.token, hub)
                .await
                .ok();
            state.last_error = Some(LocalApiFailure::from_error(&error));
            return Err(error);
        }
        state.handle = Some(new_handle);
        state.last_error = None;
        self.status_locked(&state)
    }

    pub async fn rotate_token(&self) -> Result<LocalApiStatus, LocalApiError> {
        let token = generate_local_api_token()?;
        let mut state = self.state.lock().await;
        let Some(hub) = state.hub.clone() else {
            self.config.set_string(LOCAL_API_TOKEN_CONFIG_KEY, &token)?;
            state.token = token;
            state.last_error = None;
            return self.status_locked(&state);
        };
        let port = self.configured_port()?;
        let allow_lan_connections = self.configured_allow_lan_connections()?;
        let previous_token = state.token.clone();
        let previous_handle = state.handle.take();
        if let Some(previous_handle) = previous_handle {
            self.stop_listener(previous_handle, false).await;
        }
        let new_handle = match self
            .start_listener(port, allow_lan_connections, &token, Arc::clone(&hub))
            .await
        {
            Ok(handle) => handle,
            Err(error) => {
                state.handle = self
                    .start_listener(port, allow_lan_connections, &previous_token, hub)
                    .await
                    .ok();
                state.last_error = Some(LocalApiFailure::from_error(&error));
                return Err(error);
            }
        };
        if let Err(error) = self.config.set_string(LOCAL_API_TOKEN_CONFIG_KEY, &token) {
            self.stop_listener(new_handle, false).await;
            state.handle = self
                .start_listener(port, allow_lan_connections, &previous_token, hub)
                .await
                .ok();
            state.last_error = Some(LocalApiFailure::from_error(&error));
            return Err(error);
        }
        state.token = token;
        state.handle = Some(new_handle);
        state.last_error = None;
        self.status_locked(&state)
    }

    pub async fn status(&self) -> Result<LocalApiStatus, LocalApiError> {
        let state = self.state.lock().await;
        self.status_locked(&state)
    }

    pub async fn running_generation(&self) -> Option<u64> {
        let state = self.state.lock().await;
        state.handle.as_ref().map(|_| state.session_generation)
    }

    pub async fn publish_if_generation(&self, generation: u64, room: Option<RoomState>) -> bool {
        let state = self.state.lock().await;
        if state.handle.is_none() || state.session_generation != generation {
            return false;
        }
        let Some(hub) = &state.hub else {
            return false;
        };
        hub.publish(room);
        true
    }

    async fn start_new_session(&self, state: &mut ControllerState) -> Result<(), LocalApiError> {
        let port = self.configured_port()?;
        let allow_lan_connections = self.configured_allow_lan_connections()?;
        let hub = Arc::new(ServerHub::new(self.app_version.clone()));
        let handle = self
            .start_listener(port, allow_lan_connections, &state.token, Arc::clone(&hub))
            .await?;
        state.handle = Some(handle);
        state.hub = Some(hub);
        state.session_generation = state.session_generation.saturating_add(1);
        state.last_error = None;
        Ok(())
    }

    async fn start_listener(
        &self,
        port: u16,
        allow_lan_connections: bool,
        token: &str,
        hub: Arc<ServerHub>,
    ) -> Result<ListenerHandle, LocalApiError> {
        let listener = bind_local_api_listener(port, allow_lan_connections)?;
        let bound_port = listener.local_addr()?.port();
        let id = self.next_listener_id.fetch_add(1, Ordering::AcqRel) + 1;
        let accept_cancel = CancellationToken::new();
        let session_cancel = CancellationToken::new();
        let router = build_local_api_router(LocalApiRouterState {
            policy: LocalApiAuthPolicy {
                port: bound_port,
                token: token.into(),
                allow_lan_connections,
            },
            hub,
            active_connections: Arc::clone(&self.active_connections),
            session_cancel: session_cancel.clone(),
        });
        let shutdown = accept_cancel.clone();
        let expected_shutdown = accept_cancel.clone();
        let controller_state = Arc::clone(&self.state);
        let exit_session_cancel = session_cancel.clone();
        let join = tokio::spawn(async move {
            let result = axum::serve(listener, router)
                .with_graceful_shutdown(async move { shutdown.cancelled_owned().await })
                .await;
            if expected_shutdown.is_cancelled() {
                return;
            }
            let failure = match result {
                Ok(()) => LocalApiFailure {
                    code: crate::types::LocalApiFailureCode::Io,
                    message: "Local API listener stopped unexpectedly".into(),
                    port: Some(bound_port),
                },
                Err(error) => {
                    tracing::warn!(error = %error, "Local API listener stopped with error");
                    LocalApiFailure::from_error(&LocalApiError::Bind {
                        port: bound_port,
                        source: error,
                    })
                }
            };
            exit_session_cancel.cancel();
            Self::record_listener_exit(controller_state, id, failure).await;
        });
        Ok(ListenerHandle {
            id,
            port: bound_port,
            accept_cancel,
            session_cancel,
            join,
        })
    }

    async fn record_listener_exit(
        controller_state: Arc<tokio::sync::Mutex<ControllerState>>,
        listener_id: u64,
        failure: LocalApiFailure,
    ) {
        let mut state = controller_state.lock().await;
        if !state
            .handle
            .as_ref()
            .is_some_and(|handle| handle.id == listener_id)
        {
            return;
        }
        if let Some(handle) = state.handle.take() {
            handle.accept_cancel.cancel();
            handle.session_cancel.cancel();
        }
        if let Some(hub) = state.hub.take() {
            hub.clear();
        }
        state.session_generation = state.session_generation.saturating_add(1);
        state.last_error = Some(failure);
    }

    async fn stop_listener(&self, handle: ListenerHandle, wait_for_bye: bool) {
        handle.accept_cancel.cancel();
        if !wait_for_bye {
            handle.session_cancel.cancel();
        }
        let mut join = handle.join;
        if tokio::time::timeout(Duration::from_secs(5), &mut join)
            .await
            .is_err()
        {
            handle.session_cancel.cancel();
            join.abort();
        }
    }

    fn configured_enabled(&self) -> Result<bool, LocalApiError> {
        self.config.get_bool(LOCAL_API_ENABLED_CONFIG_KEY, false)
    }

    fn configured_allow_lan_connections(&self) -> Result<bool, LocalApiError> {
        self.config.get_bool(LOCAL_API_ALLOW_LAN_CONFIG_KEY, false)
    }

    fn configured_port(&self) -> Result<u16, LocalApiError> {
        let raw = self.config.get_string(
            LOCAL_API_PORT_CONFIG_KEY,
            &DEFAULT_LOCAL_API_PORT.to_string(),
        )?;
        let port = raw.trim().parse::<u16>().unwrap_or(DEFAULT_LOCAL_API_PORT);
        if port < 1024 {
            return Ok(DEFAULT_LOCAL_API_PORT);
        }
        Ok(port)
    }

    fn status_locked(&self, state: &ControllerState) -> Result<LocalApiStatus, LocalApiError> {
        let enabled = self.configured_enabled()?;
        let allow_lan_connections = self.configured_allow_lan_connections()?;
        let configured_port = self.configured_port()?;
        let server_state = if state.handle.is_some() {
            LocalApiServerState::Running
        } else if state.last_error.is_some() {
            LocalApiServerState::Error
        } else if enabled && !state.game_running {
            LocalApiServerState::WaitingForGame
        } else if enabled {
            LocalApiServerState::Error
        } else {
            LocalApiServerState::Disabled
        };
        Ok(LocalApiStatus {
            enabled,
            allow_lan_connections,
            state: server_state,
            port: state
                .handle
                .as_ref()
                .map(|handle| handle.port)
                .unwrap_or(configured_port),
            token: state.token.clone(),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            last_error: state.last_error.clone(),
        })
    }
}

fn ensure_token(config: &dyn LocalApiConfigStore) -> Result<String, LocalApiError> {
    let existing = config.get_string(LOCAL_API_TOKEN_CONFIG_KEY, "")?;
    if !existing.trim().is_empty() {
        return Ok(existing);
    }
    let token = generate_local_api_token()?;
    config.set_string(LOCAL_API_TOKEN_CONFIG_KEY, &token)?;
    Ok(token)
}

fn validate_port(port: u16) -> Result<(), LocalApiError> {
    if port < 1024 {
        return Err(LocalApiError::InvalidPort { port });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use serde_json::Value;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    use super::*;

    #[derive(Default)]
    struct MemoryConfig {
        values: Mutex<HashMap<String, String>>,
    }

    impl LocalApiConfigStore for MemoryConfig {
        fn get_bool(&self, key: &str, default: bool) -> Result<bool, LocalApiError> {
            Ok(self
                .values
                .lock()
                .map_err(|error| LocalApiError::Config(error.to_string()))?
                .get(key)
                .and_then(|value| value.parse().ok())
                .unwrap_or(default))
        }

        fn get_string(&self, key: &str, default: &str) -> Result<String, LocalApiError> {
            Ok(self
                .values
                .lock()
                .map_err(|error| LocalApiError::Config(error.to_string()))?
                .get(key)
                .cloned()
                .unwrap_or_else(|| default.into()))
        }

        fn set_bool(&self, key: &str, value: bool) -> Result<(), LocalApiError> {
            self.set_string(key, if value { "true" } else { "false" })
        }

        fn set_string(&self, key: &str, value: &str) -> Result<(), LocalApiError> {
            self.values
                .lock()
                .map_err(|error| LocalApiError::Config(error.to_string()))?
                .insert(key.into(), value.into());
            Ok(())
        }
    }

    fn unused_port() -> u16 {
        std::net::TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn controller(config: Arc<MemoryConfig>) -> LocalApiController {
        LocalApiController::new(config, "1.2.3".into()).unwrap()
    }

    async fn connect_stream(port: u16, token: &str) -> TcpStream {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let request = format!(
            "GET /v1/stream HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Protocol: vrcx0.v1, vrcx0.token.{token}\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        while !response.ends_with(b"\r\n\r\n") {
            let mut byte = [0_u8; 1];
            stream.read_exact(&mut byte).await.unwrap();
            response.push(byte[0]);
        }
        let response = String::from_utf8(response).unwrap().to_ascii_lowercase();
        assert!(response.starts_with("http/1.1 101"));
        assert!(response.contains("sec-websocket-protocol: vrcx0.v1"));
        stream
    }

    async fn read_server_message(stream: &mut TcpStream) -> Value {
        let mut header = [0_u8; 2];
        stream.read_exact(&mut header).await.unwrap();
        assert_eq!(header[0] & 0x0f, 1);
        let mut length = u64::from(header[1] & 0x7f);
        if length == 126 {
            let mut bytes = [0_u8; 2];
            stream.read_exact(&mut bytes).await.unwrap();
            length = u64::from(u16::from_be_bytes(bytes));
        } else if length == 127 {
            let mut bytes = [0_u8; 8];
            stream.read_exact(&mut bytes).await.unwrap();
            length = u64::from_be_bytes(bytes);
        }
        let mut payload = vec![0_u8; usize::try_from(length).unwrap()];
        stream.read_exact(&mut payload).await.unwrap();
        serde_json::from_slice(&payload).unwrap()
    }

    async fn send_client_message(stream: &mut TcpStream, payload: &str) {
        let payload = payload.as_bytes();
        assert!(payload.len() < 126);
        let mask = [1_u8, 2, 3, 4];
        let mut frame = vec![0x81, 0x80 | u8::try_from(payload.len()).unwrap()];
        frame.extend_from_slice(&mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % mask.len()]),
        );
        stream.write_all(&frame).await.unwrap();
    }

    #[tokio::test]
    async fn enabled_service_waits_for_game_and_releases_port_after_stop() {
        let config = Arc::new(MemoryConfig::default());
        let port = unused_port();
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
            .unwrap();
        let controller = controller(config);

        let waiting = controller.set_enabled(true).await.unwrap();
        assert_eq!(waiting.state, LocalApiServerState::WaitingForGame);
        let running = controller.set_game_running(true).await.unwrap();
        assert_eq!(running.state, LocalApiServerState::Running);
        let stopped = controller.set_game_running(false).await.unwrap();
        assert_eq!(stopped.state, LocalApiServerState::WaitingForGame);
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_ok());
    }

    #[tokio::test]
    async fn occupied_port_keeps_enabled_false() {
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = occupied.local_addr().unwrap().port();
        let config = Arc::new(MemoryConfig::default());
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
            .unwrap();
        let controller = controller(config.clone());
        controller.set_game_running(true).await.unwrap();

        assert!(matches!(
            controller.set_enabled(true).await,
            Err(LocalApiError::PortInUse { port: error_port }) if error_port == port
        ));
        assert!(!config
            .get_bool(LOCAL_API_ENABLED_CONFIG_KEY, false)
            .unwrap());
        let status = controller.status().await.unwrap();
        assert_eq!(status.state, LocalApiServerState::Error);
    }

    #[tokio::test]
    async fn failed_running_port_change_preserves_listener_and_config() {
        let config = Arc::new(MemoryConfig::default());
        let first_port = unused_port();
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &first_port.to_string())
            .unwrap();
        let controller = controller(config.clone());
        controller.set_game_running(true).await.unwrap();
        let status = controller.set_enabled(true).await.unwrap();
        let mut stream = connect_stream(first_port, &status.token).await;
        assert_eq!(read_server_message(&mut stream).await["type"], "hello");
        assert_eq!(
            read_server_message(&mut stream).await["type"],
            "room.snapshot"
        );
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let occupied_port = occupied.local_addr().unwrap().port();

        assert!(matches!(
            controller.set_port(occupied_port).await,
            Err(LocalApiError::PortInUse { .. })
        ));
        assert_eq!(
            config.get_string(LOCAL_API_PORT_CONFIG_KEY, "").unwrap(),
            first_port.to_string()
        );
        assert_eq!(
            controller.status().await.unwrap().state,
            LocalApiServerState::Running
        );
        assert!(std::net::TcpListener::bind(("127.0.0.1", first_port)).is_err());
        send_client_message(&mut stream, r#"{"type":"resync"}"#).await;
        assert_eq!(
            read_server_message(&mut stream).await["type"],
            "room.snapshot"
        );
        controller.set_enabled(false).await.unwrap();
    }

    #[tokio::test]
    async fn successful_running_port_change_releases_old_listener() {
        let config = Arc::new(MemoryConfig::default());
        let first_port = unused_port();
        let second_port = unused_port();
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &first_port.to_string())
            .unwrap();
        let controller = controller(config);
        controller.set_game_running(true).await.unwrap();
        controller.set_enabled(true).await.unwrap();

        let changed = controller.set_port(second_port).await.unwrap();
        assert_eq!(changed.port, second_port);
        assert!(std::net::TcpListener::bind(("127.0.0.1", first_port)).is_ok());
        assert!(std::net::TcpListener::bind(("127.0.0.1", second_port)).is_err());
        controller.set_enabled(false).await.unwrap();
    }

    #[tokio::test]
    async fn same_port_is_a_no_op() {
        let config = Arc::new(MemoryConfig::default());
        let port = unused_port();
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
            .unwrap();
        let controller = controller(config.clone());

        let status = controller.set_port(port).await.unwrap();
        assert_eq!(status.port, port);
        assert_eq!(
            config.get_string(LOCAL_API_PORT_CONFIG_KEY, "").unwrap(),
            port.to_string()
        );
    }

    #[tokio::test]
    async fn stopped_port_change_probes_before_writing_config() {
        let config = Arc::new(MemoryConfig::default());
        let original_port = unused_port();
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &original_port.to_string())
            .unwrap();
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let occupied_port = occupied.local_addr().unwrap().port();
        let controller = controller(config.clone());

        assert!(matches!(
            controller.set_port(occupied_port).await,
            Err(LocalApiError::PortInUse { port }) if port == occupied_port
        ));
        assert_eq!(
            config.get_string(LOCAL_API_PORT_CONFIG_KEY, "").unwrap(),
            original_port.to_string()
        );
    }

    #[tokio::test]
    async fn automatic_start_failure_preserves_enabled_intent() {
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = occupied.local_addr().unwrap().port();
        let config = Arc::new(MemoryConfig::default());
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
            .unwrap();
        config.set_bool(LOCAL_API_ENABLED_CONFIG_KEY, true).unwrap();
        let controller = controller(config.clone());

        assert!(controller.set_game_running(true).await.is_err());
        assert!(config
            .get_bool(LOCAL_API_ENABLED_CONFIG_KEY, false)
            .unwrap());
        assert_eq!(
            controller.status().await.unwrap().state,
            LocalApiServerState::Error
        );
    }

    #[tokio::test]
    async fn changing_port_retries_a_failed_automatic_start() {
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let occupied_port = occupied.local_addr().unwrap().port();
        let replacement_port = unused_port();
        let config = Arc::new(MemoryConfig::default());
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &occupied_port.to_string())
            .unwrap();
        config.set_bool(LOCAL_API_ENABLED_CONFIG_KEY, true).unwrap();
        let controller = controller(config);

        assert!(controller.set_game_running(true).await.is_err());
        let status = controller.set_port(replacement_port).await.unwrap();
        assert_eq!(status.state, LocalApiServerState::Running);
        assert_eq!(status.port, replacement_port);
        assert!(std::net::TcpListener::bind(("127.0.0.1", replacement_port)).is_err());
        controller.set_enabled(false).await.unwrap();
    }

    #[tokio::test]
    async fn stale_generation_cannot_publish_into_a_restarted_session() {
        let config = Arc::new(MemoryConfig::default());
        let port = unused_port();
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
            .unwrap();
        let controller = controller(config);
        controller.set_game_running(true).await.unwrap();
        controller.set_enabled(true).await.unwrap();
        let previous_generation = controller.running_generation().await.unwrap();
        controller.set_enabled(false).await.unwrap();
        let status = controller.set_enabled(true).await.unwrap();

        assert!(
            !controller
                .publish_if_generation(
                    previous_generation,
                    Some(RoomState {
                        location: "wrld_stale:1".into(),
                        world_id: "wrld_stale".into(),
                        ..RoomState::default()
                    }),
                )
                .await
        );
        let mut stream = connect_stream(port, &status.token).await;
        assert_eq!(read_server_message(&mut stream).await["type"], "hello");
        assert!(read_server_message(&mut stream).await["room"].is_null());
        controller.set_enabled(false).await.unwrap();
    }

    #[tokio::test]
    async fn unexpected_listener_exit_moves_the_controller_to_error() {
        let config = Arc::new(MemoryConfig::default());
        let port = unused_port();
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
            .unwrap();
        let controller = controller(config);
        controller.set_game_running(true).await.unwrap();
        controller.set_enabled(true).await.unwrap();
        let listener_id = controller.state.lock().await.handle.as_ref().unwrap().id;

        LocalApiController::record_listener_exit(
            Arc::clone(&controller.state),
            listener_id,
            LocalApiFailure {
                code: crate::types::LocalApiFailureCode::Io,
                message: "listener failed".into(),
                port: Some(port),
            },
        )
        .await;

        let status = controller.status().await.unwrap();
        assert_eq!(status.state, LocalApiServerState::Error);
        assert_eq!(status.last_error.unwrap().message, "listener failed");
        assert!(controller.running_generation().await.is_none());
    }

    #[tokio::test]
    async fn stream_sends_initial_state_delta_resync_and_bye_before_releasing_port() {
        let config = Arc::new(MemoryConfig::default());
        let port = unused_port();
        config
            .set_string(LOCAL_API_PORT_CONFIG_KEY, &port.to_string())
            .unwrap();
        let controller = controller(config);
        controller.set_game_running(true).await.unwrap();
        let status = controller.set_enabled(true).await.unwrap();
        let mut stream = connect_stream(port, &status.token).await;

        assert_eq!(read_server_message(&mut stream).await["type"], "hello");
        let initial = read_server_message(&mut stream).await;
        assert_eq!(initial["type"], "room.snapshot");
        assert!(initial["room"].is_null());

        controller
            .publish(Some(RoomState {
                location: "wrld_a:1".into(),
                world_id: "wrld_a".into(),
                world_name: "World A".into(),
                ..RoomState::default()
            }))
            .await;
        let published = read_server_message(&mut stream).await;
        assert_eq!(published["type"], "room.snapshot");
        assert_eq!(published["room"]["worldId"], "wrld_a");

        controller
            .publish(Some(RoomState {
                location: "wrld_a:1".into(),
                world_id: "wrld_a".into(),
                world_name: "World A".into(),
                members: vec![crate::RoomMemberState {
                    user_id: "usr_a".into(),
                    display_name: "Alice".into(),
                    ..crate::RoomMemberState::default()
                }],
                ..RoomState::default()
            }))
            .await;
        let joined = read_server_message(&mut stream).await;
        assert_eq!(joined["type"], "room.joined");
        assert_eq!(joined["members"][0]["userId"], "usr_a");

        send_client_message(&mut stream, r#"{"type":"resync"}"#).await;
        let resynced = read_server_message(&mut stream).await;
        assert_eq!(resynced["type"], "room.snapshot");
        assert_eq!(resynced["seq"], joined["seq"].as_u64().unwrap() + 1);

        controller.set_game_running(false).await.unwrap();
        assert_eq!(read_server_message(&mut stream).await["type"], "bye");
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_ok());
    }
}
