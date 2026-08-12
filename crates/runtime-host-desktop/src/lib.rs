mod ancillary_snapshot;
mod app_launcher;
mod autostart;
mod background_image;
mod companion_api;
mod context;
mod game_client;
mod game_log;
mod group_order;
mod host_actions;
mod host_file_access;
mod log_watcher;
pub mod notification;
mod process_monitor;
mod registry_backup;
mod state;
pub mod vr_overlay;

pub use ancillary_snapshot::AncillaryRuntimeSnapshot;
pub use app_launcher::AppLauncherSnapshotEvent;
pub use autostart::{set_autostart_preference, AutostartPlatform};
pub use background_image::{
    background_image_files_from_paths, HostBackgroundImageFileResolver, BACKGROUND_IMAGE_EXTENSIONS,
};
pub use companion_api::DesktopCompanionApiRuntime;
pub use context::DesktopRuntimeServices;
pub use game_client::GameClientHostRuntime;
pub use game_log::GameLogHostRuntime;
pub use host_actions::{RuntimeHost, RuntimeHostActions};
pub use host_file_access::{ensure_vrchat_launch_path_allowed, is_known_root_path, HostFileAccess};
pub use log_watcher::{
    GameLogEvent, GameLogEventOrigin, GameLogEventSink, HostGameLogEventFanout,
    HostLogLocationSnapshotScanner, LogLocationSnapshot, LogWatcher,
};
pub use process_monitor::HostGameProcessMonitorActions;
pub use registry_backup::HostRegistryBackupActions;
pub use state::{
    DesktopRuntimeBundle, DesktopRuntimeHostOptions, DesktopRuntimeHostState, GameRuntimeBundle,
};
pub use vrcx_0_runtime_host::{Error, Result};
