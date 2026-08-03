mod background_capabilities;
mod game_client;
mod game_event_bus;
mod game_log;
mod log_watcher;
mod overlay_activity;
mod process_monitor;
mod registry_backup;
mod screenshots;
mod worker;

use vrcx_0_application_core::{
    sleep_interruptibly, Error, HostSessionRuntime, ImageCache, LocalGameContextSnapshot,
    LocalGameContextSource, Result, RuntimeAuthScope, RuntimeEventBus, RuntimeSyncEngine,
    TaskStopToken, TaskSupervisor, WebClient, WorldCache,
};

pub use background_capabilities::{
    build_background_discord_presence_command, build_background_presence_facts,
    presence_automation_rules_get, presence_automation_rules_set,
    run_background_presence_automation, BackgroundDiscordActivityPayload,
    BackgroundDiscordPresenceCommand, BackgroundDiscordPresenceState,
    BackgroundPresenceAutomationResult, BackgroundPresenceAutomationState, BackgroundPresenceFacts,
    BackgroundPresenceFactsInput, DiscordPresenceLabels, PresenceAutomationRuleKind,
    PresencePlayer,
};
pub use game_client::{
    DebugLoggingOutcome, DebugLoggingOutcomeKind, GameClientActions, GameClientCacheActions,
    GameClientDebugLoggingActions, GameClientLocationSource, GameClientRuntime,
    GameClientRuntimeDeps, GameClientWindowActions, NoopGameClientCacheActions,
    NoopGameClientWindowActions,
};
pub use game_event_bus::{
    AddGameLogEventPayload, CrashRelaunchDecisionPayload, EmptyEventPayload, GameClientEvent,
    GameLogPersistenceFallbackPayload, GameLogSideEffectEvent, GameNoVrPayload, NowPlayingPayload,
    RuntimeGameEventBusExt, RuntimeGameLogEventPayload, RuntimeNotificationLevel,
    RuntimeNotificationPayload, RuntimeWorkerErrorPayload, ScreenshotProcessedPayload,
};
pub use game_log::{
    duration_ms, game_log_sessions_query, instance_history_query, parse_event_time_ms, player_key,
    player_list_current_snapshot, world_id_from_location, GameLogHostActions, GameLogIngestEngine,
    GameLogIngestOptions, GameLogIngestOutput, GameLogLocalGameContextSource, GameLogProcessEvent,
    GameLogProjection, GameLogRuntime, GameLogRuntimeDeps, GameLogRuntimeState, GameLogSessionDto,
    GameLogSessionEventDto, GameLogSessionMemberDto, GameLogSessionsQueryInput, GameLogSideEffect,
    InstanceHistoryEntryOutput, InstanceHistoryQueryInput, NoopGameLogHostActions,
    PlayerListSnapshotContext, PlayerListSnapshotOutput, PlayerListSnapshotPlayer,
    PlayerListSnapshotSource, PlayerState, RuntimeSnapshot, ScreenshotInput,
};
pub use log_watcher::{
    GameLogEvent, GameLogEventSink, LogLocationSnapshot, LogLocationSnapshotScanner, LogWatcher,
    NoopLogLocationSnapshotScanner,
};
pub use overlay_activity::OverlayActivityGameIngestExt;
pub use process_monitor::{GameProcessMonitorActions, GameProcessStatus, ProcessMonitor};
pub use registry_backup::{
    registry_backup_create, registry_backup_delete, registry_backup_export_json,
    registry_backup_import_json, registry_backup_list, registry_backup_maintenance_run,
    registry_backup_restore, registry_backup_restore_prompt_acknowledge, RegistryBackupHostActions,
    RegistryBackupMaintenanceMode, RegistryBackupMaintenanceResult, RegistryBackupSnapshot,
};
pub use screenshots::{
    add_screenshot_metadata, can_decode_image, delete_all_screenshot_metadata,
    delete_text_metadata, ensure_screenshot_thumbnail, extra_screenshot_data,
    find_screenshot_search_results, find_screenshots, get_screenshot_metadata, has_vrcx_metadata,
    is_path_inside_directory, is_png_file, is_vrchat_screenshot_file_path, last_screenshot,
    list_screenshot_folder_images, list_world_screenshots, read_png_dimensions,
    screenshot_folder_tree, screenshot_metadata_json, start_screenshot_library_scan,
    write_vrcx_metadata, MetadataCacheDb, ScreenshotFolderTree, ScreenshotLibraryImage,
    ScreenshotLibraryScanStatus, ScreenshotMetadata, ScreenshotSearchResult, ScreenshotSearchType,
};
pub use worker::{OverflowPolicy, RuntimeJobHandler, RuntimePushReport};
