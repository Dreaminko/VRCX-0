mod app_update;
mod auth_credentials;
mod authenticated_runtime;
mod authenticated_session_maintenance;
mod background_capabilities;
mod batch_mutation;
mod database_upgrade;
mod database_upgrade_runtime;
mod event_payloads;
mod favorite_import;
mod favorite_transfer;
pub mod groups;
mod import_collection;
mod instance_launch;
mod local_favorites;
mod login_session;
mod media_upload;
mod moderation_sync;
mod mutual_graph_fetch;
mod noninteractive_auth;
mod note_export;
mod notification_actions;
mod prints;
mod profile_backup;
#[cfg(test)]
mod realtime;
mod share_collection;
mod shared_collection_import;
mod social_mutation;

pub use vrcx_0_application_core::{ports, vrchat_api};

pub use app_update::{
    AppUpdateBuildInfo, AppUpdateDownloadProgressPayload, AppUpdateDownloadStatusSnapshot,
    AppUpdateInstalledPayload, AppUpdateReleaseSnapshot, AppUpdateRuntime, AppUpdateStatusSnapshot,
    AppUpdateTargetResolver,
};
pub use auth_credentials::{
    delete_saved_credential, migrate_saved_credential_secrets, record_login_success, record_logout,
    saved_credential_login_start, saved_credential_session_data, saved_snapshot,
    LoginSuccessRecordInput, LogoutRecordInput, SavedCredentialLoginStartInput,
    SavedCredentialSessionData,
};
pub use authenticated_runtime::{
    AuthenticatedRuntimePhase, AuthenticatedRuntimePhaseSnapshot, AuthenticatedRuntimeStepSnapshot,
    AuthenticatedRuntimeStepStatus,
};
pub use authenticated_session_maintenance::{
    run_authenticated_session_maintenance, AuthenticatedSessionMaintenanceOutcome,
};
pub use background_capabilities::{
    refresh_background_current_user, refresh_background_group_instances,
    BackgroundCapabilitySession, BackgroundGroupInstancesRefresh,
};
pub use batch_mutation::{
    run_avatar_content_tags_batch, run_group_leave_batch, run_group_visibility_batch,
    AvatarContentTagsBatchInput, BatchMutationActions, BatchMutationItemResult,
    BatchMutationItemState, BatchMutationResult, GroupLeaveBatchInput, GroupVisibility,
    GroupVisibilityBatchInput, VrchatBatchMutationActions, BATCH_MUTATION_MAX_ITEMS,
};
pub use database_upgrade::{
    database_upgrade_preflight, run_database_upgrade, DatabaseUpgradePreflight,
    DatabaseUpgradePreflightStatus, DatabaseUpgradeRunResult, DatabaseUpgradeRunStatus,
    DatabaseUpgradeStage,
};
pub use database_upgrade_runtime::DatabaseUpgradeRuntime;
pub use favorite_import::{
    FavoriteImportItemResult, FavoriteImportItemState, FavoriteImportKind, FavoriteImportLocation,
    FavoriteImportOperation, FavoriteImportRuntime, FavoriteImportStartInput, FavoriteImportState,
    FavoriteImportStatus, FavoriteImportTarget, FAVORITE_IMPORT_MAX_ITEMS,
};
pub use favorite_transfer::{
    favorite_transfer_plan_for_item, transfer_favorites, FavoriteTransferDeps,
    FavoriteTransferInput, FavoriteTransferItem, FavoriteTransferItemResult,
    FavoriteTransferItemStatus, FavoriteTransferLocation, FavoriteTransferMode,
    FavoriteTransferResult, FavoriteTransferSource, FavoriteTransferStage, FavoriteTransferTarget,
};
pub use groups::{
    ban_member, block_group, cancel_request, create_post, delete_invite, delete_post, edit_post,
    get_audit_log_types, get_bans, get_gallery, get_group, get_group_instances,
    get_group_quick_moderation, get_invites, get_join_requests, get_logs, get_members, get_posts,
    get_user_groups, get_user_instances, join_group, kick_member, leave_group,
    respond_join_request, run_group_quick_moderation_action, search_members, send_invite,
    set_member_props, set_representation, unban_member, unblock_group, GroupApiDeps,
    GroupQuickModerationActionInput, GroupQuickModerationActionOutput, GroupQuickModerationDeps,
    GroupQuickModerationGroup, GroupQuickModerationInput, GroupQuickModerationOutput,
    VrchatGroupGalleryInput, VrchatGroupIdInput, VrchatGroupJoinRequestRespondInput,
    VrchatGroupJoinRequestsInput, VrchatGroupLogsInput, VrchatGroupMemberPropsInput,
    VrchatGroupMembersInput, VrchatGroupMembersSearchInput, VrchatGroupPagedInput,
    VrchatGroupPostCreateInput, VrchatGroupPostDeleteInput, VrchatGroupPostEditInput,
    VrchatGroupProfileInput, VrchatGroupRepresentationInput, VrchatGroupUserGroupsInput,
    VrchatGroupUserInput,
};
pub use import_collection::{preview_shared_collection, ImportPreview};
pub use instance_launch::{
    evaluate_instance_action_gates, join_instance_launch, InstanceActionGateTarget,
    InstanceActionGates, InstanceActionGatesBatchInput, InstanceActionGatesBatchOutput,
    InstanceLaunchApiFuture, InstanceLaunchDeps, InstanceLaunchHttpClient, InstanceLaunchInput,
    InstanceLaunchMode, InstanceLaunchOutcome, InstanceLaunchPipe,
};
pub use local_favorites::{
    create_local_favorite_group, delete_local_favorite_group, rename_local_favorite_group,
};
pub use login_session::{
    AutoLoginOutcome, AutoLoginStartInput, LoginApi, LoginApiFuture, LoginFailureKind,
    LoginSession, LoginSessionRuntime, LoginSessionStartBasicInput,
    LoginSessionStartCookieRestoreInput, LoginSessionStartSavedCredentialInput, LoginSessionState,
    TwoFactorMethod, WebClientLoginApi,
};
pub use media_upload::{
    prepare_media_upload_request, require_prepared_image_data, upload_legacy_entity_image,
    LegacyEntityImageKind, LegacyEntityImageUploadInput, LegacyMediaUploadDeps,
};
pub use moderation_sync::{
    refresh_player_moderations, update_player_moderation, ModerationSyncDeps,
    ModerationSyncMutationInput, ModerationSyncMutationOutput, ModerationSyncRefreshInput,
    ModerationSyncRefreshOutput, RemoteModerationRow,
};
pub use mutual_graph_fetch::{
    MutualGraphFetchCancelInput, MutualGraphFetchRuntime, MutualGraphFetchStartInput,
    MutualGraphFetchStatus,
};
pub use noninteractive_auth::{
    auth_response_error_message, current_user_from_cookie, parse_current_user_response,
    probe_current_user_from_cookie, AuthenticatedRuntimeSession, CookieSessionProbe,
    NonInteractiveAuthError,
};
pub use note_export::{
    prepare_note_export, run_note_export, NoteExportActions, NoteExportItemInput,
    NoteExportItemState, NoteExportItemStatus, NoteExportProgress, NoteExportResult,
    NoteExportStartInput, NoteExportState, NoteExportStatus, VrchatNoteExportActions,
    NOTE_EXPORT_MAX_ITEMS,
};
pub use notification_actions::{
    mark_notifications_seen_batch, NotificationMarkSeenActions, NotificationMarkSeenBatchInput,
    NotificationMarkSeenBatchItem, NotificationMarkSeenBatchResult, NotificationMarkSeenItemResult,
    NotificationMarkSeenItemState, NotificationMarkSeenLocation, VrchatNotificationMarkSeenActions,
    NOTIFICATION_MARK_SEEN_MAX_ITEMS,
};
pub use prints::{
    cleanup::{
        is_print_created_content_refresh, run_print_auto_cleanup, PrintAutoCleanupEvent,
        PrintCleanupDeps, PrintCleanupQueue, PrintCleanupQueueSink, PrintCleanupTrigger,
    },
    favorites::{favorite_state, set_print_favorite, CleanupWarningKind, PrintFavoriteState},
};
pub use profile_backup::{
    ProfileBackupActionOutcome, ProfileBackupError, ProfileBackupErrorCode, ProfileBackupKind,
    ProfileBackupOutcome, ProfileBackupPhase, ProfileBackupRuntime, ProfileBackupSettings,
    ProfileBackupState, ProfileBackupStatus, ProfileRestoreDataDisposition, ProfileRestoreFailure,
    ProfileRestoreFailureCode, ProfileRestoreProgress, ProfileRestoreProgressOperation,
    ProfileRestoreProgressPhase, ProfileRestoreResult, ProfileRestoreResultStatus,
    ProfileRestoreRollbackCleanupOutcome, ProfileRestoreRollbackState, ProfileRestoreValidation,
    ProfileRestoreValidationOutcome,
};
pub use share_collection::{
    get_or_create_share_owner_token, is_valid_share_owner_token, prepare_share_collection_payload,
    share_collection_create, share_collection_owner_hint, PreparedShareCollection,
    ShareCollectionCreateInput, ShareCollectionCreateResult, ShareCollectionDeps,
    ShareCollectionSkippedWorld, SHARE_COLLECTION_MAX_WORLDS,
};
pub use shared_collection_import::{
    prepare_shared_collection_import, run_shared_collection_import, PreparedSharedCollectionImport,
    SharedCollectionImportActions, SharedCollectionImportProgress, SharedCollectionImportResult,
    SharedCollectionImportStartInput, SharedCollectionImportState, SharedCollectionImportStatus,
    VrchatSharedCollectionImportActions, SHARED_COLLECTION_IMPORT_MAX_WORLDS,
};
pub use social_mutation::{
    accept_friend_request, cancel_friend_request, send_friend_request, unfriend,
    SocialFriendMutationInput, SocialFriendMutationOutcome, SocialFriendMutationStatus,
    SocialFriendRequestAcceptInput, SocialFriendRequestCancelInput, SocialMutationDeps,
};
pub use vrcx_0_application_core::validate_config_writes;
pub use vrcx_0_application_core::OverlayActivityInputSink;
pub use vrcx_0_application_core::{
    format_runtime_output_event, RuntimeOutputLevel, RuntimeOutputLine, RuntimeOutputMode,
};
pub use vrcx_0_application_core::{
    recommended_tokio_max_blocking_threads, recommended_tokio_max_blocking_threads_for,
    recommended_tokio_worker_threads, recommended_tokio_worker_threads_for,
};
pub use vrcx_0_application_core::{save_ugc_image_to_file, ImageCache};
pub use vrcx_0_application_core::{test_proxy_connectivity, ProxySettingsTestResult};
pub use vrcx_0_application_core::{
    BackendRuntime, BackendRuntimeMode, BackendRuntimePhase, BackendRuntimeSnapshot,
    BackendRuntimeTelemetry,
};
pub use vrcx_0_application_core::{
    Error, RuntimeDiagnostics, RuntimeEventBus, RuntimeEventSink, RuntimeVrchatAuthFailurePayload,
};
pub use vrcx_0_application_core::{GameProcessEvent, GameProcessEventSink};
pub use vrcx_0_application_core::{
    HostRealtimeSessionContext, HostSessionGameProcessStatus, HostSessionProjection,
    HostSessionRuntime, SessionHostRuntime,
};
pub use vrcx_0_application_core::{
    LocalGameContextSnapshot, LocalGameContextSource, UnavailableLocalGameContextSource,
};
pub use vrcx_0_application_core::{
    NoopUpdaterPort, UpdaterCheckRequest, UpdaterDownloadOutcome, UpdaterDownloadProgress,
    UpdaterInstallHandle, UpdaterMetadata, UpdaterPort, UpdaterProgressCallback,
};
pub use vrcx_0_application_core::{ParsedLocation, UgcCategory, WebClient, WorldCache};
pub use vrcx_0_application_core::{RuntimeAuthScope, RuntimeAuthScopeSnapshot};
pub use vrcx_0_application_core::{RuntimeBackgroundJobSnapshot, RuntimeBackgroundJobs};
pub use vrcx_0_application_core::{RuntimeLifecycle, RuntimeLifecycleSnapshot};
pub use vrcx_0_application_core::{RuntimeSyncEngine, RuntimeSyncSnapshot};
pub use vrcx_0_application_core::{
    RuntimeTask, RuntimeTaskExecutor, RuntimeTaskHandle, TaskStopToken, TaskSupervisor,
};
pub use vrcx_0_application_realtime::world_id_from_location_or_id;
pub use vrcx_0_application_realtime::{
    apply_friend_roster_baseline_sync_outcome, build_favorites_baseline,
    build_friend_roster_baseline, build_friend_roster_baseline_deferred, SocialBaselineDeps,
    SocialFavoritesBaselineInput, SocialFavoritesBaselineOutput, SocialFriendRosterBaselineInput,
    SocialFriendRosterBaselineOutput,
};
pub use vrcx_0_application_realtime::{
    is_friend_event_type, FriendBaselineCausalWatermark, FriendBaselineResult,
    FriendBaselineSyncOutcome, FriendProfileBulkLoadStatus, FriendProfileLoadStatusPayload,
    FriendProjection, FriendProjectionPatch, PendingOfflineTimerAction,
    RealtimeCurrentUserAuthority, RealtimeCurrentUserOutput, RealtimeCurrentUserProjection,
    RealtimeEntryCorrection, RealtimeEntryCorrectionFields, RealtimeEntryCorrectionStream,
    RealtimeFriendApplyResult, RealtimeFriendOutput, RealtimeFriendSnapshot,
    RealtimeFriendsRuntime, RealtimeHostRuntime, RealtimeHostRuntimeDeps,
    RealtimeInstanceClosedOutput, RealtimeInstanceClosedProjection,
    RealtimeInstanceQueueProjection, RealtimeNotificationOutput, RealtimeNotificationProjection,
    RealtimeNotificationUpsert, RealtimeProjectionSource, RealtimeSessionContext,
    RealtimeStopRequest, RealtimeTransportStartResult, RealtimeWsMessagePayload,
    RealtimeWsStatusPayload, SyntheticFriendEventOutcome,
};

pub use vrcx_0_application_core::Result;
