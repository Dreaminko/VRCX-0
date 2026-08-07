use serde::Serialize;

use vrcx_0_application::{
    AppUpdateDownloadStatusSnapshot, AppUpdateStatusSnapshot, BackgroundImageProjection,
    CommunityThemeProjection, DataDirMigrationStatus, MutualGraphFetchStatus, ProfileBackupStatus,
};
use vrcx_0_application_core::HostSessionProjection;
use vrcx_0_application_game::DebugLoggingOutcome;
use vrcx_0_host_desktop::host_capabilities::{is_host_capability_available, HostCapability};

use crate::state::DesktopRuntimeHostState;

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AncillaryRuntimeSnapshot {
    pub community_theme_state: Option<CommunityThemeProjection>,
    pub profile_backup_current_status: ProfileBackupStatus,
    pub data_dir_migration_current_status: DataDirMigrationStatus,
    pub mutual_graph_fetch_status: MutualGraphFetchStatus,
    pub app_update_status: AppUpdateStatusSnapshot,
    pub app_update_download_status: AppUpdateDownloadStatusSnapshot,
    pub game_client_debug_logging_status: Option<DebugLoggingOutcome>,
    pub game_process_snapshot: Option<HostSessionProjection>,
    pub background_image_state: BackgroundImageProjection,
}

pub async fn ancillary_runtime_snapshot(
    state: &DesktopRuntimeHostState,
) -> AncillaryRuntimeSnapshot {
    let community_theme_state = match state.desktop.community_theme.initialize().await {
        Ok(projection) => Some(projection),
        Err(error) => {
            tracing::warn!(
                error = %error,
                "failed to hydrate community theme state for ancillary runtime snapshot"
            );
            None
        }
    };
    let game_process_snapshot = if is_host_capability_available(HostCapability::GameProcessMonitor)
    {
        Some(state.runtime_context.session.projection_snapshot())
    } else {
        None
    };

    AncillaryRuntimeSnapshot {
        community_theme_state,
        profile_backup_current_status: state.profile_backup.current_status(),
        data_dir_migration_current_status: state.data_dir_migration.current_status(),
        mutual_graph_fetch_status: state.runtime_context.mutual_graph_fetch.status(),
        app_update_status: state.desktop.app_update.hydration_snapshot(),
        app_update_download_status: state.desktop.app_update.download_status(),
        game_client_debug_logging_status: state.game.game_client_runtime.debug_logging_outcome(),
        game_process_snapshot,
        background_image_state: state.desktop.background_image.projection(),
    }
}
