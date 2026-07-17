import type {
    BackendRuntimeSnapshot,
    BackendRuntimeTelemetry,
    FriendProfileLoadStatusPayload,
    FriendProjection,
    GameLogProjection,
    HostSessionProjection,
    OverlayActivitySnapshot,
    PrintAutoCleanupEvent,
    RealtimeCurrentUserProjection,
    RealtimeEntryCorrection,
    RealtimeInstanceClosedProjection,
    RealtimeInstanceQueueProjection,
    RealtimeNotificationProjection
} from '@/platform/tauri/bindings';

import type {
    ProfileBackupStatus,
    ProfileRestoreProgress
} from '../profileBackupService';

export type RuntimeEventName =
    | 'addGameLogEvent'
    | 'backendRuntimeTelemetry'
    | 'gameLogProjection'
    | 'gameLogPersistenceFallback'
    | 'gameLogSideEffect'
    | 'gameClientEvent'
    | 'runtimeWorkerError'
    | 'runtimeVrchatAuthFailure'
    | 'runtimeGroupInstancesProjection'
    | 'overlayActivitySnapshot'
    | 'printsAutoCleanup'
    | 'profileBackupStatus'
    | 'profileRestoreProgress'
    | 'favoritesChanged'
    | 'friendProfileLoadStatus'
    | 'realtimeFriendProjection'
    | 'realtimeUserProjection'
    | 'realtimeEntryCorrection'
    | 'realtimeNotificationProjection'
    | 'realtimeCurrentUserProjection'
    | 'realtimeInstanceClosedProjection'
    | 'realtimeInstanceQueueProjection'
    | 'updateIsGameRunning'
    | 'browserFocus';

export type FavoritesChangedEventPayload = {
    kind: string;
    local: boolean;
    remote: boolean;
};

export type RuntimeGroupInstance = Record<string, unknown> & {
    id?: string;
    instanceId?: string;
    location?: string;
    worldId?: string;
};

export type RuntimeGroupInstancesProjection = {
    status: string;
    userId: string;
    endpoint: string;
    fetchedAt?: string | null;
    error?: string | null;
    instances?: RuntimeGroupInstance[];
    groupOrder?: string[];
};

export type RuntimeVrchatAuthFailurePayload = {
    ownerUserId: string;
    endpoint: string;
    path: string;
    reason: string;
    statusCode: number;
    authScopeGeneration: number;
};

export type RuntimeEventPayloadMap = {
    addGameLogEvent: unknown;
    backendRuntimeTelemetry: BackendRuntimeTelemetry;
    gameLogProjection: GameLogProjection;
    gameLogPersistenceFallback: unknown;
    gameLogSideEffect: unknown;
    gameClientEvent: unknown;
    runtimeWorkerError: unknown;
    runtimeVrchatAuthFailure: RuntimeVrchatAuthFailurePayload;
    runtimeGroupInstancesProjection: RuntimeGroupInstancesProjection;
    overlayActivitySnapshot: OverlayActivitySnapshot;
    printsAutoCleanup: PrintAutoCleanupEvent;
    profileBackupStatus: ProfileBackupStatus;
    profileRestoreProgress: ProfileRestoreProgress;
    favoritesChanged: FavoritesChangedEventPayload;
    friendProfileLoadStatus: FriendProfileLoadStatusPayload;
    realtimeFriendProjection: FriendProjection;
    realtimeUserProjection: unknown;
    realtimeEntryCorrection: RealtimeEntryCorrection;
    realtimeNotificationProjection: RealtimeNotificationProjection;
    realtimeCurrentUserProjection: RealtimeCurrentUserProjection;
    realtimeInstanceClosedProjection: RealtimeInstanceClosedProjection;
    realtimeInstanceQueueProjection: RealtimeInstanceQueueProjection;
    updateIsGameRunning: HostSessionProjection;
    browserFocus: unknown;
};

export type RuntimeSnapshotPayload =
    | BackendRuntimeSnapshot
    | Record<string, unknown>
    | null;
