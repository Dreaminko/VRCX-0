use vrcx_0_application_core::RuntimeEventBus;

use super::OverlayActivitySnapshot;

pub trait RuntimeOverlayActivityEventBusExt {
    fn emit_overlay_activity_snapshot(&self, payload: OverlayActivitySnapshot);
}

impl RuntimeOverlayActivityEventBusExt for RuntimeEventBus {
    fn emit_overlay_activity_snapshot(&self, payload: OverlayActivitySnapshot) {
        self.emit("overlayActivitySnapshot", payload);
    }
}
