use std::net::{SocketAddr, SocketAddrV4};
use std::sync::{atomic::AtomicU32, Arc, Mutex};

use axum::body::Body;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, HOST, SEC_WEBSOCKET_PROTOCOL};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use http::HeaderMap;
use serde_json::json;
use socket2::{Domain, Protocol, Socket, Type as SocketType};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::auth::{
    authorize_companion_api_request, CompanionApiAuthError, CompanionApiAuthPolicy,
    BASE_SUBPROTOCOL,
};
use crate::session::run_session;
use crate::state::{diff_room, RoomChange, RoomState};
use crate::wire::ByeReason;
use crate::{CompanionApiError, PROTOCOL_VERSION};

const BROADCAST_CAPACITY: usize = 32;

#[derive(Clone, Debug)]
pub(crate) enum ServerEvent {
    Snapshot {
        revision: u64,
        room: Option<Arc<RoomState>>,
        at: String,
    },
    Changes {
        revision: u64,
        changes: Vec<RoomChange>,
        at: String,
    },
    Bye(ByeReason),
}

pub(crate) struct ServerHub {
    app_version: String,
    state: Mutex<ServerHubState>,
    events: broadcast::Sender<ServerEvent>,
}

#[derive(Clone, Debug)]
pub(crate) struct ServerHubSnapshot {
    pub(crate) revision: u64,
    pub(crate) room: Option<Arc<RoomState>>,
}

struct ServerHubState {
    revision: u64,
    room: Option<Arc<RoomState>>,
}

impl ServerHub {
    pub(crate) fn new(app_version: String) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            app_version,
            state: Mutex::new(ServerHubState {
                revision: 0,
                room: None,
            }),
            events,
        }
    }

    pub(crate) fn app_version(&self) -> &str {
        &self.app_version
    }

    pub(crate) fn snapshot(&self) -> ServerHubSnapshot {
        self.state
            .lock()
            .map(|state| ServerHubSnapshot {
                revision: state.revision,
                room: state.room.clone(),
            })
            .unwrap_or(ServerHubSnapshot {
                revision: 0,
                room: None,
            })
    }

    pub(crate) fn subscribe(&self) -> (broadcast::Receiver<ServerEvent>, ServerHubSnapshot) {
        match self.state.lock() {
            Ok(state) => (
                self.events.subscribe(),
                ServerHubSnapshot {
                    revision: state.revision,
                    room: state.room.clone(),
                },
            ),
            Err(error) => {
                tracing::warn!(error = %error, "Companion API room state lock failed");
                (
                    self.events.subscribe(),
                    ServerHubSnapshot {
                        revision: 0,
                        room: None,
                    },
                )
            }
        }
    }

    pub(crate) fn publish(&self, room: Option<RoomState>) {
        let at = now_iso();
        let room = room.map(Arc::new);
        match self.state.lock() {
            Ok(mut state) => {
                let changes = match (state.room.as_deref(), room.as_ref()) {
                    (None, None) => Vec::new(),
                    (Some(_), None) => {
                        state.revision = state.revision.saturating_add(1);
                        state.room = None;
                        let _ = self.events.send(ServerEvent::Snapshot {
                            revision: state.revision,
                            room: None,
                            at,
                        });
                        return;
                    }
                    (None, Some(next)) => vec![RoomChange::Snapshot(Arc::clone(next))],
                    (Some(previous), Some(next)) => {
                        diff_room(Some(previous), Arc::clone(next))
                    }
                };
                if changes.is_empty() {
                    return;
                }
                state.revision = state.revision.saturating_add(1);
                state.room = room;
                if let [RoomChange::Snapshot(snapshot)] = changes.as_slice() {
                    let _ = self.events.send(ServerEvent::Snapshot {
                        revision: state.revision,
                        room: Some(snapshot.clone()),
                        at,
                    });
                } else {
                    let _ = self.events.send(ServerEvent::Changes {
                        revision: state.revision,
                        changes,
                        at,
                    });
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "Companion API room state lock failed");
            }
        }
    }

    pub(crate) fn send_bye(&self, reason: ByeReason) {
        let _ = self.events.send(ServerEvent::Bye(reason));
    }

    pub(crate) fn clear(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.room = None;
            state.revision = 0;
        }
    }
}

#[derive(Clone)]
pub(crate) struct CompanionApiRouterState {
    pub(crate) policy: CompanionApiAuthPolicy,
    pub(crate) hub: Arc<ServerHub>,
    pub(crate) active_connections: Arc<AtomicU32>,
    pub(crate) session_cancel: CancellationToken,
}

pub(crate) fn build_companion_api_router(state: CompanionApiRouterState) -> Router {
    Router::new()
        .route("/v1/health", get(companion_api_health))
        .route("/v1/stream", get(companion_api_stream))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            companion_api_auth_middleware,
        ))
        .with_state(state)
}

async fn companion_api_health() -> impl IntoResponse {
    axum::Json(json!({ "ok": true, "protocol": PROTOCOL_VERSION }))
}

async fn companion_api_stream(
    State(state): State<CompanionApiRouterState>,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket
        .protocols([BASE_SUBPROTOCOL])
        .on_upgrade(move |socket| run_session(socket, state))
}

async fn companion_api_auth_middleware(
    State(state): State<CompanionApiRouterState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let authorization = header_to_str(request.headers(), AUTHORIZATION.as_str());
    let host = header_to_str(request.headers(), HOST.as_str());
    let subprotocols = header_to_str(request.headers(), SEC_WEBSOCKET_PROTOCOL.as_str());
    match authorize_companion_api_request(&state.policy, authorization, host, subprotocols) {
        Ok(_) => next.run(request).await,
        Err(CompanionApiAuthError::InvalidHost) => {
            (StatusCode::FORBIDDEN, "forbidden").into_response()
        }
        Err(CompanionApiAuthError::Unauthorized) => {
            (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
        }
    }
}

fn header_to_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

pub(crate) fn bind_companion_api_listener(
    port: u16,
    allow_lan_connections: bool,
) -> Result<TcpListener, CompanionApiError> {
    let socket = Socket::new(Domain::IPV4, SocketType::STREAM, Some(Protocol::TCP))
        .map_err(|error| bind_error(port, error))?;
    #[cfg(not(windows))]
    socket
        .set_reuse_address(true)
        .map_err(|error| bind_error(port, error))?;
    let address = if allow_lan_connections {
        [0, 0, 0, 0]
    } else {
        [127, 0, 0, 1]
    };
    socket
        .bind(&SocketAddr::V4(SocketAddrV4::new(address.into(), port)).into())
        .map_err(|error| bind_error(port, error))?;
    socket
        .listen(1024)
        .map_err(|error| bind_error(port, error))?;
    socket
        .set_nonblocking(true)
        .map_err(|error| bind_error(port, error))?;
    TcpListener::from_std(socket.into()).map_err(|error| bind_error(port, error))
}

fn bind_error(port: u16, source: std::io::Error) -> CompanionApiError {
    if source.kind() == std::io::ErrorKind::AddrInUse {
        CompanionApiError::PortInUse { port }
    } else {
        CompanionApiError::Bind { port, source }
    }
}

pub(crate) fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(location: &str) -> RoomState {
        RoomState {
            location: location.into(),
            world_id: location.split(':').next().unwrap_or_default().into(),
            ..RoomState::default()
        }
    }

    #[tokio::test]
    async fn subscription_snapshot_and_events_keep_their_exact_revisions() {
        let hub = ServerHub::new("1.2.3".into());
        hub.publish(Some(room("wrld_a:1")));
        let (mut receiver, initial) = hub.subscribe();
        assert_eq!(initial.revision, 1);
        assert_eq!(initial.room.unwrap().world_id, "wrld_a");

        hub.publish(Some(room("wrld_b:2")));
        hub.publish(Some(room("wrld_c:3")));

        let ServerEvent::Snapshot { revision, room, .. } = receiver.recv().await.unwrap() else {
            panic!("expected a snapshot event");
        };
        assert_eq!(revision, 2);
        assert_eq!(room.unwrap().world_id, "wrld_b");
        assert_eq!(hub.snapshot().revision, 3);
    }
}
