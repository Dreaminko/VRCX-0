import type { UserProfileEntity } from '@/domain/entities/profileEntities';
import type { FeedLiveEntryPayload } from '@/domain/feed/feedLiveTypes';
import type {
    FriendProjection,
    RealtimeCurrentUserProjection,
    RealtimeEntryCorrection,
    RealtimeInstanceClosedProjection,
    RealtimeNotificationProjection,
    RealtimeNotificationUpsert,
    RealtimeUserProjection
} from '@/platform/tauri/bindings';
import type { NotificationRow } from '@/repositories/notificationPersistenceRepository';

export type RealtimeFriendProjectionPayload = Omit<
    FriendProjection,
    'feedEntries'
> & {
    feedEntries?: FeedLiveEntryPayload[];
};

export type RealtimeEntryCorrectionPayload = Omit<
    RealtimeEntryCorrection,
    'fields'
> & {
    fields: {
        displayName?: string;
        worldName?: string;
        displayLocation?: string;
    };
};

export type RealtimeUserRecord = UserProfileEntity & {
    endpoint?: string;
    updatedAt?: string;
    userId?: string;
};

export type RealtimeUserProjectionPayload = Omit<
    RealtimeUserProjection,
    'users'
> & {
    users: RealtimeUserRecord[];
};

export type RealtimeGameStatePatch = Record<string, unknown> &
    Partial<{
        currentLocation: string;
        currentWorldId: string;
        currentWorldName: string;
        currentDestination: string;
        currentLocationStartedAt: string | null;
        currentLocationPlayerIds: [];
        currentLocationPlayers: [];
        lastGameLogAt?: string;
        lastGameLogType?: 'location';
    }>;

export type RealtimeCurrentUserProjectionPayload = Omit<
    RealtimeCurrentUserProjection,
    'patch' | 'snapshot' | 'gameStatePatch'
> & {
    patch: UserProfileEntity;
    snapshot: UserProfileEntity;
    gameStatePatch?: RealtimeGameStatePatch | null;
};

export type RealtimeNotificationUpsertPayload = Omit<
    RealtimeNotificationUpsert,
    'notification' | 'insertDefaults'
> & {
    notification: NotificationRow;
    insertDefaults?: NotificationRow | null;
};

export type RealtimeNotificationProjectionPayload = Omit<
    RealtimeNotificationProjection,
    'upserts'
> & {
    upserts?: RealtimeNotificationUpsertPayload[];
};

export type RealtimeInstanceClosedProjectionPayload = Omit<
    RealtimeInstanceClosedProjection,
    'notification' | 'feedEntry'
> & {
    notification: NotificationRow;
    feedEntry: FeedLiveEntryPayload;
};
