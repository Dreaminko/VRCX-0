use std::sync::{
    mpsc::{self, RecvTimeoutError},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use vrcx_0_vr_overlay::{OverlaySurfaceId, RgbaFrame};

#[cfg(all(feature = "steamvr-overlay", any(windows, target_os = "linux")))]
use super::openvr_backend::OpenVrOverlayBackend;
#[cfg(all(feature = "openxr-overlay", any(windows, target_os = "linux")))]
use super::openxr_backend::OpenXrOverlayBackend;
use super::{
    command::{OverlayCommandError, OverlayServiceCommand},
    noop::NoopOverlayBackend,
    status::{OverlayServicePhase, OverlayServiceStatus},
    types::{BackendStartError, OverlaySurfaceConfig, VrDeviceSnapshot},
};

const OVERLAY_TICK_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickOutcome {
    Continue,
    RuntimeQuit,
}

pub trait OverlayBackend: Send + 'static {
    fn start(&mut self) -> Result<(), BackendStartError>;
    fn register_surface(&mut self, config: OverlaySurfaceConfig) -> Result<(), String>;
    fn unregister_surface(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String> {
        self.hide(surface_id)
    }
    fn update_frame(
        &mut self,
        surface_id: &OverlaySurfaceId,
        frame: RgbaFrame,
    ) -> Result<(), String>;
    fn show(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String>;
    fn hide(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String>;
    fn set_alpha(&mut self, _surface_id: &OverlaySurfaceId, _alpha: f32) -> Result<(), String> {
        Ok(())
    }
    fn snapshot_devices(&mut self) -> Result<Vec<VrDeviceSnapshot>, String>;
    fn tick(&mut self) -> TickOutcome {
        TickOutcome::Continue
    }
    fn stop(&mut self);
}

#[derive(Clone)]
pub struct OverlayActorHandle {
    sender: mpsc::Sender<OverlayActorMessage>,
    status: Arc<Mutex<OverlayServiceStatus>>,
    runtime_quit_at: Arc<Mutex<Option<Instant>>>,
}

enum OverlayActorMessage {
    Command(OverlayCommandEnvelope),
    SnapshotDevices {
        reply: mpsc::Sender<Result<Vec<VrDeviceSnapshot>, OverlayCommandError>>,
    },
}

struct OverlayCommandEnvelope {
    command: OverlayServiceCommand,
    reply: mpsc::Sender<Result<(), OverlayCommandError>>,
}

impl OverlayActorHandle {
    pub fn spawn_noop() -> Self {
        Self::spawn_with_backend(NoopOverlayBackend)
    }

    #[cfg(all(feature = "steamvr-overlay", any(windows, target_os = "linux")))]
    pub fn spawn_openvr() -> Self {
        Self::spawn_with_backend(OpenVrOverlayBackend::new())
    }

    #[cfg(all(feature = "openxr-overlay", any(windows, target_os = "linux")))]
    pub fn spawn_openxr() -> Self {
        Self::spawn_with_backend(OpenXrOverlayBackend::new())
    }

    #[cfg(test)]
    pub fn spawn_for_test<B>(backend: B) -> Self
    where
        B: OverlayBackend,
    {
        Self::spawn_with_backend(backend)
    }

    pub fn spawn_with_backend<B>(backend: B) -> Self
    where
        B: OverlayBackend,
    {
        let (sender, receiver) = mpsc::channel::<OverlayActorMessage>();
        let status = Arc::new(Mutex::new(OverlayServiceStatus::default()));
        let runtime_quit_at = Arc::new(Mutex::new(None));
        let actor_status = Arc::clone(&status);
        let actor_runtime_quit_at = Arc::clone(&runtime_quit_at);
        thread::Builder::new()
            .name("vrcx-vr-overlay".to_string())
            .spawn(move || run_actor(backend, receiver, actor_status, actor_runtime_quit_at))
            .expect("spawn VR overlay actor thread");
        Self {
            sender,
            status,
            runtime_quit_at,
        }
    }

    pub fn send(&self, command: OverlayServiceCommand) -> Result<(), OverlayCommandError> {
        let (reply, result) = mpsc::channel();
        self.sender
            .send(OverlayActorMessage::Command(OverlayCommandEnvelope {
                command,
                reply,
            }))
            .map_err(|_| OverlayCommandError::Stopped)?;
        result.recv().map_err(|_| OverlayCommandError::Stopped)?
    }

    pub fn snapshot_devices(&self) -> Result<Vec<VrDeviceSnapshot>, OverlayCommandError> {
        let (reply, result) = mpsc::channel();
        self.sender
            .send(OverlayActorMessage::SnapshotDevices { reply })
            .map_err(|_| OverlayCommandError::Stopped)?;
        result.recv().map_err(|_| OverlayCommandError::Stopped)?
    }

    pub fn status(&self) -> OverlayServiceStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn runtime_quit_at(&self) -> Option<Instant> {
        self.runtime_quit_at
            .lock()
            .map(|slot| *slot)
            .unwrap_or(None)
    }
}

fn run_actor<B>(
    mut backend: B,
    receiver: mpsc::Receiver<OverlayActorMessage>,
    status: Arc<Mutex<OverlayServiceStatus>>,
    runtime_quit_at: Arc<Mutex<Option<Instant>>>,
) where
    B: OverlayBackend,
{
    let mut skip_backend_stop = false;
    let mut last_tick_at = Instant::now();
    loop {
        match receiver.recv_timeout(OVERLAY_TICK_INTERVAL) {
            Ok(message) => {
                match message {
                    OverlayActorMessage::Command(envelope) => {
                        let should_stop = matches!(envelope.command, OverlayServiceCommand::Stop);
                        let result = handle_command(&mut backend, envelope.command, &status);
                        let _ = envelope.reply.send(result);
                        if should_stop {
                            skip_backend_stop = true;
                            break;
                        }
                    }
                    OverlayActorMessage::SnapshotDevices { reply } => {
                        let result = backend
                            .snapshot_devices()
                            .map_err(|error| record_backend_error(&status, error));
                        let _ = reply.send(result);
                    }
                }
                if run_tick_if_due(&mut backend, &status, &runtime_quit_at, &mut last_tick_at) {
                    skip_backend_stop = true;
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if run_tick_if_due(&mut backend, &status, &runtime_quit_at, &mut last_tick_at) {
                    skip_backend_stop = true;
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    if !skip_backend_stop {
        backend.stop();
        update_status(&status, OverlayServicePhase::Stopped, None);
    }
}

fn run_tick_if_due<B>(
    backend: &mut B,
    status: &Arc<Mutex<OverlayServiceStatus>>,
    runtime_quit_at: &Arc<Mutex<Option<Instant>>>,
    last_tick_at: &mut Instant,
) -> bool
where
    B: OverlayBackend,
{
    if last_tick_at.elapsed() < OVERLAY_TICK_INTERVAL {
        return false;
    }
    *last_tick_at = Instant::now();
    run_tick(backend, status, runtime_quit_at)
}

fn run_tick<B>(
    backend: &mut B,
    status: &Arc<Mutex<OverlayServiceStatus>>,
    runtime_quit_at: &Arc<Mutex<Option<Instant>>>,
) -> bool
where
    B: OverlayBackend,
{
    if !actor_is_running(status) {
        return false;
    }
    match backend.tick() {
        TickOutcome::Continue => false,
        TickOutcome::RuntimeQuit => {
            if let Ok(mut slot) = runtime_quit_at.lock() {
                *slot = Some(Instant::now());
            }
            update_status(
                status,
                OverlayServicePhase::Stopped,
                Some("VR runtime requested quit".to_string()),
            );
            true
        }
    }
}

fn actor_is_running(status: &Arc<Mutex<OverlayServiceStatus>>) -> bool {
    status
        .lock()
        .map(|status| status.phase == OverlayServicePhase::Running)
        .unwrap_or(false)
}

fn handle_command<B>(
    backend: &mut B,
    command: OverlayServiceCommand,
    status: &Arc<Mutex<OverlayServiceStatus>>,
) -> Result<(), OverlayCommandError>
where
    B: OverlayBackend,
{
    match command {
        OverlayServiceCommand::Start => {
            update_status(status, OverlayServicePhase::Starting, None);
            if let Err(error) = backend.start() {
                update_status(
                    status,
                    OverlayServicePhase::Error,
                    Some(error.message.clone()),
                );
                backend.stop();
                return Err(if error.permanent {
                    OverlayCommandError::BackendUnsupported(error.message)
                } else {
                    OverlayCommandError::Backend(error.message)
                });
            }
            update_status(status, OverlayServicePhase::Running, None);
            Ok(())
        }
        OverlayServiceCommand::RegisterSurface(config) => backend
            .register_surface(config)
            .map_err(|error| record_backend_error(status, error)),
        OverlayServiceCommand::RegisterOptionalSurface(config) => backend
            .register_surface(config)
            .map_err(OverlayCommandError::Backend),
        OverlayServiceCommand::UnregisterSurface(surface_id) => backend
            .unregister_surface(&surface_id)
            .map_err(|error| record_backend_error(status, error)),
        OverlayServiceCommand::UpdateFrame { surface_id, frame } => {
            validate_frame(&frame).inspect_err(|error| {
                update_status(status, OverlayServicePhase::Error, Some(error.to_string()));
            })?;
            backend
                .update_frame(&surface_id, frame)
                .map_err(|error| record_backend_error(status, error))
        }
        OverlayServiceCommand::Show(surface_id) => backend
            .show(&surface_id)
            .map_err(|error| record_backend_error(status, error)),
        OverlayServiceCommand::Hide(surface_id) => backend
            .hide(&surface_id)
            .map_err(|error| record_backend_error(status, error)),
        OverlayServiceCommand::SetAlpha { surface_id, alpha } => backend
            .set_alpha(&surface_id, alpha)
            .map_err(|error| record_backend_error(status, error)),
        OverlayServiceCommand::Stop => {
            backend.stop();
            update_status(status, OverlayServicePhase::Stopped, None);
            Ok(())
        }
    }
}

fn validate_frame(frame: &RgbaFrame) -> Result<(), OverlayCommandError> {
    let expected = RgbaFrame::expected_byte_len(frame.size)
        .ok_or(OverlayCommandError::InvalidFrameDimensions)?;
    if frame.data.len() != expected {
        return Err(OverlayCommandError::InvalidFrameLength {
            expected,
            actual: frame.data.len(),
        });
    }
    Ok(())
}

fn record_backend_error(
    status: &Arc<Mutex<OverlayServiceStatus>>,
    error: String,
) -> OverlayCommandError {
    update_status(status, OverlayServicePhase::Error, Some(error.clone()));
    OverlayCommandError::Backend(error)
}

fn update_status(
    status: &Arc<Mutex<OverlayServiceStatus>>,
    phase: OverlayServicePhase,
    last_error: Option<String>,
) {
    if let Ok(mut status) = status.lock() {
        status.phase = phase;
        status.last_error = last_error;
    }
}
