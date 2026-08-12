use std::sync::Arc;
use std::time::Duration;

use vrcx_0_application_core::LocalGameContextSource;
use vrcx_0_application_realtime::{FavoriteBaselineSnapshot, FriendProjectionObserver};

use crate::{GroupOrderSource, RuntimeHostState};

pub type RuntimeHostCallback = Arc<dyn Fn() + Send + Sync>;
pub type RuntimeHostFavoritesCallback = Arc<dyn Fn(&FavoriteBaselineSnapshot) + Send + Sync>;

pub trait RuntimeHostProfileExtension: Send + Sync {
    fn start_profile_services(&self, _state: &RuntimeHostState) {}

    fn stop_profile_services(&self) {}

    fn start_profile_maintenance(&self, _state: &RuntimeHostState) {}

    fn clear_profile_session(&self) {}

    fn profile_session_scope_changed(&self) {
        self.clear_profile_session();
    }

    fn wait_for_profile_maintenance_stopped(&self, _timeout: Duration) -> bool {
        true
    }
}

pub struct RuntimeHostComposition {
    pub local_game_context: Arc<dyn LocalGameContextSource>,
    pub group_order_source: Arc<dyn GroupOrderSource>,
    pub friend_note_change_sink: Option<RuntimeHostCallback>,
    pub favorites_sink: Option<RuntimeHostFavoritesCallback>,
    pub friend_projection_observer: Option<Arc<dyn FriendProjectionObserver>>,
    pub profile_extension: Option<Arc<dyn RuntimeHostProfileExtension>>,
}
