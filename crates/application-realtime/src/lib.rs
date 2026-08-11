mod realtime;
mod social_baseline;

pub use realtime::lifecycle_log as realtime_lifecycle_log;
mod world_enrich;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_support {
    pub use crate::realtime::service::test_support::{
        runtime_with_active_session, TestDir, TestRealtimeHostRuntime,
    };
}

pub use realtime::{
    is_friend_event_type, is_print_created_content_refresh, FriendBaselineCausalWatermark,
    FriendBaselineResult, FriendBaselineSyncOutcome, FriendProfileBulkLoadStatus,
    FriendProfileLoadStatusPayload, FriendProjection, FriendProjectionPatch,
    FriendStateBucketAuthority, PendingOfflineTimerAction, RealtimeCurrentUserAuthority,
    RealtimeCurrentUserGameLogContext, RealtimeCurrentUserOutput, RealtimeCurrentUserProjection,
    RealtimeCurrentUserRefreshExpectation, RealtimeEntryCorrection, RealtimeEntryCorrectionFields,
    RealtimeEntryCorrectionStream, RealtimeFeedPatch, RealtimeFeedProjection, RealtimeFeedUpsert,
    RealtimeFriendApplyResult, RealtimeFriendOutput, RealtimeFriendRosterSnapshot,
    RealtimeFriendSnapshot, RealtimeFriendsRuntime, RealtimeHostRuntime, RealtimeHostRuntimeDeps,
    RealtimeInstanceClosedOutput, RealtimeInstanceClosedProjection, RealtimeInstanceQueueKind,
    RealtimeInstanceQueueProjection, RealtimeNotificationOutput, RealtimeNotificationProjection,
    RealtimeNotificationUpsert, RealtimeSessionContext, RealtimeStopRequest,
    RealtimeTransportLifecycleEvent, RealtimeTransportStartResult, RealtimeTransportTermination,
    RealtimeUserProjection, RealtimeWsMessagePayload, RealtimeWsStatus, RealtimeWsStatusPayload,
    SyntheticFriendEventOutcome, UserQueryCachePolicy, UserQueryKind, UserQueryOptions,
};
pub use realtime::{normalize_v1_notification, normalize_v2_notification};
pub use realtime::{
    CURRENT_USER_AVATAR_RESPONSE_AUTHORITY_FIELDS,
    CURRENT_USER_FALLBACK_AVATAR_RESPONSE_AUTHORITY_FIELDS,
};
pub use social_baseline::{
    apply_friend_roster_baseline_sync_outcome, build_favorites_baseline,
    build_favorites_baseline_from_friend_ids, build_favorites_baseline_from_friend_records,
    build_friend_roster_baseline, build_friend_roster_baseline_deferred,
    build_synced_friend_roster_baseline, FavoriteBaselineSnapshot, FavoriteGroupOutput,
    FriendStatusVerdicts, SocialBaselineDeps, SocialFavoritesBaselineInput,
    SocialFavoritesBaselineOutput, SocialFavoritesBaselineRequest,
    SocialFriendRosterBaselineInput, SocialFriendRosterBaselineOutput, SyncedFriendRosterBaseline,
};
pub use world_enrich::world_id_from_location_or_id;
