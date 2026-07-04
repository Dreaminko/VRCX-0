use std::time::{Duration, Instant};

use vrcx_0_host::vr_overlay::{OverlaySurfaceConfig, VrDeviceSnapshot};
use vrcx_0_vr_overlay::{OverlaySurfaceId, RgbaFrame};

use super::{
    eligibility::VrOverlayEligibility,
    service::{OverlayBackendPreference, VrOverlayServiceControl},
};

const OVERLAY_START_RETRY_INITIAL_BACKOFF: Duration = Duration::from_secs(5);
const OVERLAY_START_RETRY_MAX_BACKOFF: Duration = Duration::from_secs(60);

pub struct VrOverlayManager<S> {
    service: S,
    next_start_attempt_at: Option<Instant>,
    start_retry_backoff: Duration,
    unsupported_eligibility: Option<VrOverlayEligibility>,
}

impl<S> VrOverlayManager<S>
where
    S: VrOverlayServiceControl,
{
    pub fn new(service: S) -> Self {
        Self {
            service,
            next_start_attempt_at: None,
            start_retry_backoff: OVERLAY_START_RETRY_INITIAL_BACKOFF,
            unsupported_eligibility: None,
        }
    }

    pub fn reconcile(&mut self, eligibility: VrOverlayEligibility) {
        if eligibility.can_run() {
            if self
                .unsupported_eligibility
                .is_some_and(|blocked| blocked == eligibility)
            {
                return;
            }
            self.unsupported_eligibility = None;
            if !self.service.is_running() {
                let now = Instant::now();
                if self
                    .next_start_attempt_at
                    .is_some_and(|next_attempt| now < next_attempt)
                {
                    return;
                }
                match self.service.start() {
                    Ok(()) => {
                        self.reset_retry_state();
                    }
                    Err(error) if error.permanent => {
                        self.reset_retry_state();
                        self.unsupported_eligibility = Some(eligibility);
                        tracing::warn!(
                            error = %error.message,
                            "VR overlay backend is unsupported by the current VR runtime; \
                             retrying once VR conditions change"
                        );
                    }
                    Err(error) => {
                        self.next_start_attempt_at = Some(now + self.start_retry_backoff);
                        self.start_retry_backoff =
                            (self.start_retry_backoff * 2).min(OVERLAY_START_RETRY_MAX_BACKOFF);
                        log_overlay_start_error(&error.message);
                    }
                }
            } else {
                self.reset_retry_state();
            }
        } else {
            self.reset_retry_state();
            self.unsupported_eligibility = None;
            if self.service.is_running() {
                self.service.stop();
            }
        }
    }

    fn reset_retry_state(&mut self) {
        self.next_start_attempt_at = None;
        self.start_retry_backoff = OVERLAY_START_RETRY_INITIAL_BACKOFF;
    }

    pub fn is_running(&self) -> bool {
        self.service.is_running()
    }

    pub fn update_frame(&mut self, frame: RgbaFrame) -> Result<(), String> {
        self.service.update_frame(frame)
    }

    pub fn update_surface_frame(
        &mut self,
        surface_id: &OverlaySurfaceId,
        frame: RgbaFrame,
    ) -> Result<(), String> {
        self.service.update_surface_frame(surface_id, frame)
    }

    pub fn show(&mut self) -> Result<(), String> {
        self.service.show()
    }

    pub fn show_surface(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String> {
        self.service.show_surface(surface_id)
    }

    pub fn hide_surface(&mut self, surface_id: &OverlaySurfaceId) -> Result<(), String> {
        self.service.hide_surface(surface_id)
    }

    pub fn set_surface_alpha(
        &mut self,
        surface_id: &OverlaySurfaceId,
        alpha: f32,
    ) -> Result<(), String> {
        self.service.set_surface_alpha(surface_id, alpha)
    }

    pub fn snapshot_devices(&mut self) -> Result<Vec<VrDeviceSnapshot>, String> {
        self.service.snapshot_devices()
    }

    pub fn set_surface_configs(
        &mut self,
        configs: Vec<OverlaySurfaceConfig>,
    ) -> Result<(), String> {
        self.service.set_surface_configs(configs)
    }

    pub fn set_backend_preference(&mut self, preference: OverlayBackendPreference) {
        self.unsupported_eligibility = None;
        self.reset_retry_state();
        self.service.set_backend_preference(preference);
    }

    pub fn active_backend(&self) -> Option<&'static str> {
        self.service.active_backend()
    }

    pub fn into_inner(self) -> S {
        self.service
    }
}

fn log_overlay_start_error(error: &str) {
    if !is_expected_overlay_start_wait(error) {
        tracing::warn!(error = %error, "failed to start VR overlay service");
        return;
    }
    if error.contains("cooling down") {
        tracing::debug!(
            error = %error,
            "VR overlay start deferred by runtime quit cooldown"
        );
    } else {
        tracing::debug!(
            error = %error,
            "VR overlay service is waiting for the OpenVR server"
        );
    }
}

fn is_expected_overlay_start_wait(error: &str) -> bool {
    error.contains("VRInitError_Init_NoServerForBackgroundApp") || error.contains("cooling down")
}

#[cfg(test)]
mod tests {
    use super::is_expected_overlay_start_wait;

    #[test]
    fn expected_overlay_start_wait_matches_openvr_server_and_cooldown_errors() {
        assert!(is_expected_overlay_start_wait(
            "OpenVR init failed: VRInitError_Init_NoServerForBackgroundApp"
        ));
        assert!(is_expected_overlay_start_wait(
            "VR runtime quit 120ms ago; cooling down"
        ));
        assert!(!is_expected_overlay_start_wait(
            "OpenVR init failed: VRInitError_Init_InterfaceNotFound"
        ));
    }
}
