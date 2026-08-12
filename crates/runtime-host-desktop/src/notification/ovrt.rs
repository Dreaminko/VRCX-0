use vrcx_0_host_desktop::overlay_notifications::OvrToolkit;
use vrcx_0_runtime_host::notification::RenderedNotification;

use super::NotificationDeliveryPlan;

const NOTIFICATION_APP_TITLE: &str = "VRCX-0";

pub(super) fn send_ovrt_notification(
    ovrt: &OvrToolkit,
    plan: NotificationDeliveryPlan,
    render: &RenderedNotification,
    local_image: Option<&str>,
) {
    ovrt.send_notification(
        plan.ovrt_hud,
        plan.ovrt_wrist,
        NOTIFICATION_APP_TITLE,
        &render.text,
        local_image,
    );
}
