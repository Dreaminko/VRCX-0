use serde::Serialize;

pub trait RuntimeEventPayload: Serialize + specta::Type {
    const EVENT_NAME: &'static str;
}

#[macro_export]
macro_rules! runtime_event_payload {
    ($payload:ty, $event:literal) => {
        impl $crate::RuntimeEventPayload for $payload {
            const EVENT_NAME: &'static str = $event;
        }
    };
}

runtime_event_payload!(
    vrcx_0_core::realtime::RealtimeWsStatusPayload,
    "realtimeWsStatus"
);
runtime_event_payload!(
    vrcx_0_core::screenshots::ScreenshotLibraryScanStatus,
    "screenshotLibraryScanStatus"
);

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::RuntimeEventPayload;

    #[derive(Serialize, specta::Type)]
    struct TestPayload;

    crate::runtime_event_payload!(TestPayload, "testPayload");

    #[test]
    fn macro_and_core_payloads_keep_their_event_names() {
        assert_eq!(TestPayload::EVENT_NAME, "testPayload");
        assert_eq!(
            vrcx_0_core::realtime::RealtimeWsStatusPayload::EVENT_NAME,
            "realtimeWsStatus"
        );
        assert_eq!(
            vrcx_0_core::screenshots::ScreenshotLibraryScanStatus::EVENT_NAME,
            "screenshotLibraryScanStatus"
        );
    }
}
