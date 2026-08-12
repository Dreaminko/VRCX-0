use std::sync::{Arc, Mutex, Weak};

use vrcx_0_application_activity::OverlayActivityDelivery;
use vrcx_0_application_core::{WebClient, WorldCache};
use vrcx_0_application_realtime::RealtimeHostRuntime;
use vrcx_0_core::location::{format_display_location, is_meaningful_world_name, parse_location};

#[derive(Clone, Default)]
pub struct RealtimeUserImageResolverSlot {
    inner: Arc<Mutex<Weak<RealtimeHostRuntime>>>,
}

impl RealtimeUserImageResolverSlot {
    pub fn set(&self, runtime: &Arc<RealtimeHostRuntime>) {
        match self.inner.lock() {
            Ok(mut slot) => {
                *slot = Arc::downgrade(runtime);
            }
            Err(error) => {
                tracing::warn!("failed to set realtime user image resolver bridge: {error}");
            }
        }
    }

    pub fn cached_url(
        &self,
        endpoint: &str,
        user_id: &str,
        allow_user_icon: bool,
    ) -> Option<String> {
        let runtime = self.inner.lock().ok()?.upgrade()?;
        runtime.cached_user_notification_image_url(endpoint, user_id, allow_user_icon)
    }
}

pub async fn resolve_delivery_world_name(
    world_cache: &WorldCache,
    web: &WebClient,
    endpoint: &str,
    delivery: &OverlayActivityDelivery,
) -> Option<(String, String)> {
    if is_meaningful_world_name(&delivery.entry.content.world_name) {
        return None;
    }
    let world_id = {
        let content = &delivery.entry.content;
        let explicit = content.world_id.trim();
        if explicit.is_empty() {
            parse_location(&content.location).world_id
        } else {
            explicit.to_string()
        }
    };
    if world_id.is_empty() {
        return None;
    }
    let name = world_cache.resolve_name(web, endpoint, &world_id).await?;
    let parsed = parse_location(&delivery.entry.content.location);
    let display_location =
        format_display_location(&parsed, &name, &delivery.entry.content.group_name);
    Some((name, display_location))
}
