import { toast } from 'sonner';

import { commands } from '@/platform/tauri/bindings';
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
import { tauriClient } from '@/platform/tauri/client';
import mediaRepository from '@/repositories/vrchatMediaRepository';
import { printCleanupWarningMessageKey } from '@/shared/utils/printFavoriteMessages';
import { normalizeString } from '@/shared/utils/string';
import { normalizeVrchatEndpointDomain } from '@/shared/vrchatEndpoint';
import {
    type FavoriteRevisionKind,
    useFavoriteRevisionStore
} from '@/state/favoriteRevisionStore';
import type { FavoriteKind } from '@/state/favoriteStoreTypes';
import { useNotificationStore } from '@/state/notificationStore';
import { usePrintFavoriteStore } from '@/state/printFavoriteStore';
import {
    createGroupInstancesState,
    useRuntimeStore
} from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';

import { handleRuntimeAuthFailure } from './authSessionRecoveryService';
import { resumeFrontendSessionFromBackendRuntime } from './backendRuntimeSessionResumeService';
import { bindDeepLinkEvents, drainPendingDeepLinks } from './deepLinkService';
import { refreshLocalFavoritesForKinds } from './favoriteLocalRefreshService';
import {
    applyFriendProfileLoadStatusPayload,
    isFriendProfileLoadTerminalStatus
} from './friendProfileLoadService';
import { recordRuntimeGameClientEvent } from './gameClientLifecycle';
import { applyRuntimeGameLogProjection } from './gameLogIngestService';
import { handleGameRunningUpdate } from './gameStateService';
import { isHostCapabilityAvailable } from './hostCapabilityService';
import i18n from './i18nService';
import { handleRealtimeInstanceQueueProjection } from './realtimeInstanceQueueService';
import {
    handleRealtimeCurrentUserProjection,
    handleRealtimeEntryCorrection,
    handleRealtimeFriendProjection,
    handleRealtimeInstanceClosedProjection,
    handleRealtimeNotificationProjection,
    handleRealtimeUserCacheProjection
} from './realtimePresenceService';
import { pushSharedFeedNotification } from './sharedFeedNotificationService';
import { showSQLiteErrorDialog } from './sqliteErrorDialogService';
import { handleBrowserFocus } from './vrcStatusService';

type RuntimeEventName =
    | 'addGameLogEvent'
    | 'backendRuntimeTelemetry'
    | 'gameLogProjection'
    | 'gameLogPersistenceFallback'
    | 'gameLogSideEffect'
    | 'gameClientEvent'
    | 'runtimeWorkerError'
    | 'runtimeGroupInstancesProjection'
    | 'overlayActivitySnapshot'
    | 'printsAutoCleanup'
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

type FavoritesChangedEventPayload = {
    kind: string;
    local: boolean;
    remote: boolean;
};

type RuntimeEventPayloadMap = {
    addGameLogEvent: unknown;
    backendRuntimeTelemetry: BackendRuntimeTelemetry;
    gameLogProjection: GameLogProjection;
    gameLogPersistenceFallback: unknown;
    gameLogSideEffect: unknown;
    gameClientEvent: unknown;
    runtimeWorkerError: unknown;
    runtimeGroupInstancesProjection: RuntimeGroupInstancesProjection;
    overlayActivitySnapshot: OverlayActivitySnapshot;
    printsAutoCleanup: PrintAutoCleanupEvent;
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

type RuntimeSnapshotPayload =
    | BackendRuntimeSnapshot
    | Record<string, unknown>
    | null;

type RuntimeEventUnsubscribe = () => void;

type RuntimeGroupInstance = Record<string, unknown> & {
    id?: string;
    instanceId?: string;
    location?: string;
    worldId?: string;
};

type RuntimeGroupInstancesProjection = {
    status: string;
    userId: string;
    endpoint: string;
    fetchedAt?: string | null;
    error?: string | null;
    instances?: RuntimeGroupInstance[];
    groupOrder?: string[];
};

let backendRuntimeHydrationPromise: Promise<void> | null = null;
let pendingBackendRuntimeHydrationSnapshot: RuntimeSnapshotPayload = null;
let hasPendingBackendRuntimeHydrationSnapshot = false;
type BackendRealtimeProjectionScope = {
    userId: string;
    generation: number;
};
let pendingBackendRealtimeProjectionEvents: Array<{
    name: RuntimeEventName;
    payload: unknown;
    scope: BackendRealtimeProjectionScope;
}> = [];
const FRIEND_PROFILE_PROJECTION_BATCH_MS = 10_000;
const FRIEND_PROFILE_BULK_LOAD_SOURCE = 'friendProfileBulkLoad';
let friendProfileProjectionBatchTimer: ReturnType<typeof setTimeout> | null =
    null;
let pendingFriendProfileProjection: FriendProjection | null = null;
let pendingFriendProfileUsers: unknown[] = [];

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function friendProfileLoadIsActive(): boolean {
    const status = useRuntimeStore.getState().friendProfileLoad.status;
    return status === 'running' || status === 'cancelling';
}

function isBatchableFriendProfileProjection(
    projection: FriendProjection
): boolean {
    return (
        (projection.patches?.length ?? 0) > 0 &&
        (projection.removals?.length ?? 0) === 0 &&
        (projection.feedEntries?.length ?? 0) === 0 &&
        !projection.friendLogChanged
    );
}

function scheduleFriendProfileProjectionBatch(): void {
    if (friendProfileProjectionBatchTimer !== null) {
        return;
    }
    friendProfileProjectionBatchTimer = setTimeout(() => {
        friendProfileProjectionBatchTimer = null;
        flushFriendProfileProjectionBatch();
    }, FRIEND_PROFILE_PROJECTION_BATCH_MS);
}

function clearFriendProfileProjectionBatchTimer(): void {
    if (friendProfileProjectionBatchTimer === null) {
        return;
    }
    clearTimeout(friendProfileProjectionBatchTimer);
    friendProfileProjectionBatchTimer = null;
}

function queueFriendProfileProjection(projection: FriendProjection): void {
    if (
        pendingFriendProfileProjection &&
        (pendingFriendProfileProjection.generation !== projection.generation ||
            pendingFriendProfileProjection.baselineRevision !==
                projection.baselineRevision)
    ) {
        flushFriendProfileProjectionBatch();
    }
    if (pendingFriendProfileProjection) {
        pendingFriendProfileProjection = {
            ...pendingFriendProfileProjection,
            patches: [
                ...(pendingFriendProfileProjection.patches ?? []),
                ...(projection.patches ?? [])
            ]
        };
    } else {
        pendingFriendProfileProjection = {
            ...projection,
            patches: [...(projection.patches ?? [])],
            removals: [],
            feedEntries: [],
            friendLogChanged: false
        };
    }
    scheduleFriendProfileProjectionBatch();
}

function queueFriendProfileUsers(payload: unknown): boolean {
    const projection = isRecord(payload) ? payload : {};
    if (!Array.isArray(projection.users) || projection.users.length === 0) {
        return false;
    }
    pendingFriendProfileUsers.push(...projection.users);
    scheduleFriendProfileProjectionBatch();
    return true;
}

function flushFriendProfileProjectionBatch(): void {
    clearFriendProfileProjectionBatchTimer();
    const friendProjection = pendingFriendProfileProjection;
    const users = pendingFriendProfileUsers;
    pendingFriendProfileProjection = null;
    pendingFriendProfileUsers = [];
    if (users.length > 0) {
        deliverBackendRealtimeProjectionEvent('realtimeUserProjection', {
            users
        });
    }
    if (friendProjection) {
        deliverBackendRealtimeProjectionEvent(
            'realtimeFriendProjection',
            friendProjection
        );
    }
}

function resetFriendProfileProjectionBatch(): void {
    clearFriendProfileProjectionBatchTimer();
    pendingFriendProfileProjection = null;
    pendingFriendProfileUsers = [];
}

function queueFriendProfileLoadProjection(
    name: RuntimeEventName,
    payload: unknown
): boolean {
    if (
        !friendProfileLoadIsActive() ||
        !isRecord(payload) ||
        payload.source !== FRIEND_PROFILE_BULK_LOAD_SOURCE
    ) {
        return false;
    }
    if (name === 'realtimeUserProjection') {
        return queueFriendProfileUsers(payload);
    }
    if (name !== 'realtimeFriendProjection') {
        return false;
    }
    const projection =
        payload as RuntimeEventPayloadMap['realtimeFriendProjection'];
    if (!isBatchableFriendProfileProjection(projection)) {
        return false;
    }
    queueFriendProfileProjection(projection);
    return true;
}

function applyBackendRuntimeSnapshot(
    snapshot: RuntimeSnapshotPayload,
    {
        markHydrated = true,
        applyFriendProfileLoad = false
    }: { markHydrated?: boolean; applyFriendProfileLoad?: boolean } = {}
) {
    const runtimeStore = useRuntimeStore.getState();
    runtimeStore.setBackendRuntimeSnapshot(snapshot);
    if (
        applyFriendProfileLoad &&
        isRecord(snapshot) &&
        isRecord(snapshot.friendProfileLoad)
    ) {
        applyFriendProfileLoadStatusPayload(
            snapshot.friendProfileLoad as FriendProfileLoadStatusPayload
        );
    }
    if (markHydrated) {
        runtimeStore.setShellState({
            backendRuntimeSnapshotHydrated: true
        });
    }
}

function hydrateBackendRuntimeSnapshot(
    snapshot: RuntimeSnapshotPayload
): Promise<void> {
    pendingBackendRuntimeHydrationSnapshot = snapshot;
    hasPendingBackendRuntimeHydrationSnapshot = true;

    if (!backendRuntimeHydrationPromise) {
        useRuntimeStore.getState().setShellState({
            backendRuntimeSessionHydrating: true
        });
        backendRuntimeHydrationPromise = (async () => {
            while (hasPendingBackendRuntimeHydrationSnapshot) {
                const nextSnapshot = pendingBackendRuntimeHydrationSnapshot;
                pendingBackendRuntimeHydrationSnapshot = null;
                hasPendingBackendRuntimeHydrationSnapshot = false;
                applyBackendRuntimeSnapshot(nextSnapshot, {
                    markHydrated: false,
                    applyFriendProfileLoad: true
                });
                try {
                    await resumeFrontendSessionFromBackendRuntime(nextSnapshot);
                    handleBackendRuntimeAuthFailureSnapshot(nextSnapshot);
                    flushPendingBackendRealtimeProjectionEvents();
                } catch (error) {
                    console.warn(
                        'Failed to resume frontend session from backend runtime:',
                        error
                    );
                }
            }
        })().finally(() => {
            useRuntimeStore.getState().setShellState({
                backendRuntimeSnapshotHydrated: true,
                backendRuntimeSessionHydrating: false
            });
            backendRuntimeHydrationPromise = null;
        });
    }
    return backendRuntimeHydrationPromise;
}

function publishNowPlayingSharedFeed(payload: Record<string, unknown>): void {
    const videoUrl = normalizeString(payload.videoUrl || payload.url);
    if (!videoUrl) {
        return;
    }

    const videoName = normalizeString(payload.videoName || payload.name);
    const displayName = normalizeString(payload.displayName);
    const message = [
        videoName || videoUrl,
        displayName ? `(${displayName})` : ''
    ]
        .filter(Boolean)
        .join(' ');

    pushSharedFeedNotification({
        ...payload,
        created_at:
            normalizeString(payload.created_at) ||
            normalizeString(payload.startedAt) ||
            new Date().toISOString(),
        type: 'VideoPlay',
        videoUrl,
        videoName,
        videoId: normalizeString(payload.videoId || payload.source),
        location: normalizeString(payload.location),
        displayName,
        userId: normalizeString(payload.userId),
        message,
        notyName: message
    }).catch((error: unknown) => {
        console.warn(
            'Failed to publish runtime video shared feed notification:',
            error
        );
    });
}

function recordGameLogPersistenceTelemetry(
    name: RuntimeEventName,
    payload: unknown
): void {
    useRuntimeStore.getState().recordRuntimeEvent(name, payload);
    const record = isRecord(payload) ? payload : {};
    const errorMessage = normalizeString(record.error);
    if (errorMessage) {
        console.warn('Backend GameLog persistence failed:', errorMessage);
    }
}

function isBackendRuntimeRealtimeOwner(): boolean {
    const runtimeState = useRuntimeStore.getState();
    const sessionState = useSessionStore.getState();
    const snapshot = isRecord(runtimeState.backendRuntime)
        ? runtimeState.backendRuntime
        : {};
    const authUserId = normalizeString(snapshot.authUserId);
    return Boolean(
        snapshot.phase === 'running' &&
        snapshot.authStatus === 'authenticated' &&
        snapshot.wsStatus !== 'authFailure' &&
        snapshot.mode !== 'headless' &&
        authUserId &&
        runtimeState.auth.currentUserId === authUserId &&
        sessionState.sessionPhase === 'ready'
    );
}

function isBackendRuntimeRealtimeCandidate(): boolean {
    const snapshot = useRuntimeStore.getState().backendRuntime;
    return Boolean(
        isRecord(snapshot) &&
        snapshot.phase === 'running' &&
        snapshot.authStatus === 'authenticated' &&
        snapshot.wsStatus !== 'authFailure' &&
        snapshot.mode !== 'headless' &&
        normalizeString(snapshot.authUserId)
    );
}

function currentBackendRealtimeUserId(): string {
    const snapshot = useRuntimeStore.getState().backendRuntime;
    return isRecord(snapshot) ? normalizeString(snapshot.authUserId) : '';
}

function projectionGeneration(payload: unknown): number {
    const generation = Number(isRecord(payload) ? payload.generation : null);
    return Number.isFinite(generation) && generation > 0 ? generation : 0;
}

function currentBackendRealtimeProjectionScope(
    payload: unknown
): BackendRealtimeProjectionScope | null {
    const userId = currentBackendRealtimeUserId();
    const generation = projectionGeneration(payload);
    if (!userId || !generation) {
        return null;
    }
    return { userId, generation };
}

function sameBackendRealtimeProjectionScope(
    left: BackendRealtimeProjectionScope | null,
    right: BackendRealtimeProjectionScope | null
): boolean {
    return Boolean(
        left &&
        right &&
        left.userId === right.userId &&
        left.generation === right.generation
    );
}

function isRealtimeProjectionEvent(name: RuntimeEventName): boolean {
    return (
        name === 'realtimeFriendProjection' ||
        name === 'realtimeUserProjection' ||
        name === 'realtimeNotificationProjection' ||
        name === 'realtimeCurrentUserProjection' ||
        name === 'realtimeInstanceClosedProjection' ||
        name === 'realtimeInstanceQueueProjection'
    );
}

function handleBackendRealtimeProjectionFailure(error: unknown): void {
    showSQLiteErrorDialog(error).catch((dialogError: unknown) => {
        console.warn('Realtime SQLite error dialog failed:', dialogError);
    });
    useNotificationStore.getState().pushNotification({
        level: 'warning',
        title: 'Realtime event failed',
        message: error instanceof Error ? error.message : String(error)
    });
}

function deliverBackendRealtimeProjectionEvent(
    name: RuntimeEventName,
    payload: unknown
): void {
    useRuntimeStore.getState().recordRuntimeEvent(name, payload);
    if (name === 'realtimeFriendProjection') {
        handleRealtimeFriendProjection(
            payload as RuntimeEventPayloadMap['realtimeFriendProjection']
        );
    } else if (name === 'realtimeUserProjection') {
        handleRealtimeUserCacheProjection(payload);
    } else if (name === 'realtimeNotificationProjection') {
        Promise.resolve(
            handleRealtimeNotificationProjection(
                payload as RuntimeEventPayloadMap['realtimeNotificationProjection']
            )
        ).catch(handleBackendRealtimeProjectionFailure);
    } else if (name === 'realtimeCurrentUserProjection') {
        handleRealtimeCurrentUserProjection(
            payload as RuntimeEventPayloadMap['realtimeCurrentUserProjection']
        );
    } else if (name === 'realtimeInstanceClosedProjection') {
        Promise.resolve(
            handleRealtimeInstanceClosedProjection(
                payload as RuntimeEventPayloadMap['realtimeInstanceClosedProjection']
            )
        ).catch(handleBackendRealtimeProjectionFailure);
    } else if (name === 'realtimeInstanceQueueProjection') {
        handleRealtimeInstanceQueueProjection(payload);
    }
}

function queuePendingBackendRealtimeProjectionEvent(
    name: RuntimeEventName,
    payload: unknown
): void {
    const scope = currentBackendRealtimeProjectionScope(payload);
    if (!scope) {
        return;
    }
    const currentScope =
        pendingBackendRealtimeProjectionEvents[0]?.scope ?? null;
    if (
        pendingBackendRealtimeProjectionEvents.length &&
        !sameBackendRealtimeProjectionScope(currentScope, scope)
    ) {
        pendingBackendRealtimeProjectionEvents = [];
    }
    pendingBackendRealtimeProjectionEvents.push({ name, payload, scope });
    if (pendingBackendRealtimeProjectionEvents.length > 128) {
        pendingBackendRealtimeProjectionEvents.shift();
    }
}

function flushPendingBackendRealtimeProjectionEvents(): void {
    const currentScope =
        pendingBackendRealtimeProjectionEvents[0]?.scope ?? null;
    if (
        !pendingBackendRealtimeProjectionEvents.length ||
        !isBackendRuntimeRealtimeOwner() ||
        currentScope?.userId !== currentBackendRealtimeUserId()
    ) {
        return;
    }
    const pending = pendingBackendRealtimeProjectionEvents;
    pendingBackendRealtimeProjectionEvents = [];
    for (const entry of pending) {
        if (sameBackendRealtimeProjectionScope(entry.scope, currentScope)) {
            deliverBackendRealtimeProjectionEvent(entry.name, entry.payload);
        }
    }
}

function prunePendingBackendRealtimeProjectionEvents(
    snapshot: RuntimeSnapshotPayload
): void {
    if (!pendingBackendRealtimeProjectionEvents.length) {
        return;
    }
    const userId = isRecord(snapshot)
        ? normalizeString(snapshot.authUserId)
        : '';
    const active = Boolean(
        isRecord(snapshot) &&
        snapshot.phase === 'running' &&
        snapshot.authStatus === 'authenticated' &&
        snapshot.mode !== 'headless' &&
        userId
    );
    const currentScope = pendingBackendRealtimeProjectionEvents[0]?.scope;
    if (!active || currentScope?.userId !== userId) {
        pendingBackendRealtimeProjectionEvents = [];
    }
}

function isBackendRuntimeAuthFailureSnapshot(
    snapshot: RuntimeSnapshotPayload
): boolean {
    return Boolean(
        isRecord(snapshot) &&
        snapshot.phase === 'running' &&
        snapshot.authStatus === 'authenticated' &&
        normalizeString(snapshot.authUserId) &&
        normalizeString(snapshot.wsStatus) === 'authFailure'
    );
}

function handleBackendRuntimeAuthFailureSnapshot(
    snapshot: RuntimeSnapshotPayload
): void {
    if (!isBackendRuntimeAuthFailureSnapshot(snapshot)) {
        return;
    }

    const error = Object.assign(new Error('Backend realtime auth failed.'), {
        status: 401,
        endpoint: 'auth',
        payload: { snapshot }
    });
    const handled = handleRuntimeAuthFailure(error);
    if (handled) {
        handled.catch((recoveryError: unknown) => {
            console.warn(
                'Backend runtime auth failure recovery failed:',
                recoveryError
            );
        });
    }
}

function handleBackendRealtimeProjectionEvent(
    name: RuntimeEventName,
    payload: unknown
): boolean {
    if (!isRealtimeProjectionEvent(name)) {
        return false;
    }
    if (!isBackendRuntimeRealtimeOwner()) {
        if (isBackendRuntimeRealtimeCandidate()) {
            queuePendingBackendRealtimeProjectionEvent(name, payload);
        }
        return true;
    }

    flushPendingBackendRealtimeProjectionEvents();
    if (queueFriendProfileLoadProjection(name, payload)) {
        return true;
    }
    if (
        name === 'realtimeFriendProjection' ||
        name === 'realtimeUserProjection'
    ) {
        flushFriendProfileProjectionBatch();
    }
    deliverBackendRealtimeProjectionEvent(name, payload);
    return true;
}

function requestGameRunningStateRefresh(source: string): void {
    if (!isHostCapabilityAvailable('gameProcessMonitor')) {
        return;
    }

    commands.appCheckGameRunning().catch((error: unknown) => {
        console.warn(
            `Game process state refresh failed during ${source}:`,
            error
        );
    });
}

function requestGroupInstancesRefresh(source: string): void {
    commands.appRuntimeGroupInstancesRefresh().catch((error: unknown) => {
        console.warn(
            `Runtime group instances refresh failed during ${source}:`,
            error
        );
    });
}

let lastPrintCleanupWarning: string | null = null;

function showPrintCleanupToast(event: PrintAutoCleanupEvent): void {
    const warningKey = printCleanupWarningMessageKey(event.warning);
    if (warningKey) {
        if (event.warning !== lastPrintCleanupWarning) {
            lastPrintCleanupWarning = event.warning ?? null;
            toast.warning(
                i18n.t(warningKey, {
                    remaining: event.remaining
                })
            );
        }
        return;
    }

    lastPrintCleanupWarning = null;
    if (event.deleted > 0) {
        toast.success(
            i18n.t('view.tools.prints_favorites.cleanup_deleted', {
                count: event.deleted,
                remaining: event.remaining
            })
        );
    }
}

function refreshPrintFavoritesAfterCleanup(): void {
    mediaRepository
        .getPrintFavorites()
        .then((state) => {
            usePrintFavoriteStore.getState().hydratePrintFavorites(state);
        })
        .catch((error: unknown) => {
            console.warn(
                'Failed to refresh print favorites after cleanup:',
                error
            );
        });
}

function normalizeFavoritesChangedKind(kind: string): FavoriteRevisionKind {
    return kind === 'friend' || kind === 'world' || kind === 'avatar'
        ? kind
        : 'unknown';
}

function handleFavoritesChangedEvent(
    payload: FavoritesChangedEventPayload
): void {
    const kind = normalizeFavoritesChangedKind(payload.kind);
    useFavoriteRevisionStore.getState().bumpRevision({
        kind,
        remote: Boolean(payload.remote)
    });
    if (!payload.local) {
        return;
    }
    const kinds: FavoriteKind[] =
        kind === 'unknown' ? ['friend', 'world', 'avatar'] : [kind];
    refreshLocalFavoritesForKinds(kinds).catch((error: unknown) => {
        console.warn('Failed to refresh local favorites after change:', error);
    });
}

function handleRuntimeEvent(
    name: RuntimeEventName,
    payload: RuntimeEventPayloadMap[RuntimeEventName]
): void {
    const runtimeStore = useRuntimeStore.getState();

    if (name === 'gameLogPersistenceFallback') {
        recordGameLogPersistenceTelemetry(name, payload);
        return;
    }

    if (name === 'friendProfileLoadStatus') {
        const friendProfileLoad =
            payload as RuntimeEventPayloadMap['friendProfileLoadStatus'];
        if (isFriendProfileLoadTerminalStatus(friendProfileLoad.status)) {
            flushFriendProfileProjectionBatch();
        }
        runtimeStore.recordRuntimeEvent(name, payload);
        applyFriendProfileLoadStatusPayload(friendProfileLoad);
        return;
    }

    if (name === 'printsAutoCleanup') {
        const printCleanupEvent =
            payload as RuntimeEventPayloadMap['printsAutoCleanup'];
        runtimeStore.recordRuntimeEvent(name, payload);
        usePrintFavoriteStore.getState().applyPrintCleanup(printCleanupEvent);
        refreshPrintFavoritesAfterCleanup();
        showPrintCleanupToast(printCleanupEvent);
        return;
    }

    if (name === 'favoritesChanged') {
        runtimeStore.recordRuntimeEvent(name, payload);
        handleFavoritesChangedEvent(
            payload as RuntimeEventPayloadMap['favoritesChanged']
        );
        return;
    }

    if (handleBackendRealtimeProjectionEvent(name, payload)) {
        return;
    }

    runtimeStore.recordRuntimeEvent(name, payload);

    if (name === 'backendRuntimeTelemetry') {
        const record = isRecord(payload) ? payload : {};
        const snapshot = isRecord(record.snapshot) ? record.snapshot : null;
        prunePendingBackendRealtimeProjectionEvents(snapshot);
        if (!useRuntimeStore.getState().shell.backendRuntimeSnapshotHydrated) {
            hydrateBackendRuntimeSnapshot(snapshot);
        } else {
            applyBackendRuntimeSnapshot(snapshot);
            resumeFrontendSessionFromBackendRuntime(snapshot)
                .catch((error: unknown) => {
                    console.warn(
                        'Failed to resume frontend session from backend runtime:',
                        error
                    );
                })
                .then(() => {
                    handleBackendRuntimeAuthFailureSnapshot(snapshot);
                    flushPendingBackendRealtimeProjectionEvents();
                });
        }
        return;
    }

    if (name === 'realtimeEntryCorrection') {
        handleRealtimeEntryCorrection(
            payload as RuntimeEventPayloadMap['realtimeEntryCorrection']
        );
        return;
    }

    if (name === 'gameLogProjection') {
        if (!isHostCapabilityAvailable('runtimeGameLogIngest')) {
            return;
        }
        applyRuntimeGameLogProjection(payload);
        return;
    }

    if (name === 'gameLogSideEffect') {
        if (!isHostCapabilityAvailable('runtimeGameLogSideEffects')) {
            return;
        }
        const record = isRecord(payload) ? payload : {};
        const kind = String(record.kind || '');
        const sidePayload = isRecord(record.payload) ? record.payload : {};
        if (kind === 'nowPlaying') {
            runtimeStore.setNowPlayingState(sidePayload);
            publishNowPlayingSharedFeed(sidePayload);
        } else if (kind === 'nowPlayingReset') {
            runtimeStore.resetNowPlayingState();
        } else if (kind === 'screenshotProcessed') {
            runtimeStore.setGameState({
                lastScreenshotPath: String(sidePayload.path || '')
            });
        } else if (kind === 'gameNoVR') {
            runtimeStore.setGameState({
                isGameNoVR: Boolean(sidePayload.isGameNoVR)
            });
        } else if (kind === 'notification') {
            useNotificationStore.getState().pushNotification(sidePayload);
        }
        return;
    }

    if (name === 'runtimeGroupInstancesProjection') {
        const record =
            payload as RuntimeEventPayloadMap['runtimeGroupInstancesProjection'];
        const status = normalizeString(record.status) || 'ready';
        const userId = normalizeString(record.userId);
        const endpoint = normalizeString(record.endpoint);
        const auth = useRuntimeStore.getState().auth;
        const currentUserId = normalizeString(auth.currentUserId);
        const currentEndpoint = normalizeString(auth.currentUserEndpoint);
        if (!currentUserId || !userId) {
            if (status === 'idle') {
                runtimeStore.setGroupInstancesState(
                    createGroupInstancesState()
                );
            }
            return;
        }
        if (
            userId !== currentUserId ||
            normalizeVrchatEndpointDomain(endpoint) !==
                normalizeVrchatEndpointDomain(currentEndpoint)
        ) {
            return;
        }
        const instances = Array.isArray(record.instances)
            ? record.instances
            : undefined;
        const groupOrder = Array.isArray(record.groupOrder)
            ? record.groupOrder
            : undefined;
        const patch: Record<string, unknown> = {
            status,
            userId: currentUserId,
            endpoint: currentEndpoint,
            lastLoadedAt: new Date().toISOString(),
            error: normalizeString(record.error)
        };
        if (instances) {
            patch.instances = instances;
        }
        if (groupOrder) {
            patch.groupOrder = groupOrder;
        }
        if (record.fetchedAt) {
            patch.fetchedAt = record.fetchedAt;
        }
        runtimeStore.setGroupInstancesState(patch);
        return;
    }

    if (name === 'gameClientEvent') {
        if (!isHostCapabilityAvailable('runtimeGameClientLifecycle')) {
            return;
        }
        const record = isRecord(payload) ? payload : {};
        const kind = String(record.kind || '');
        const clientPayload = isRecord(record.payload) ? record.payload : {};
        recordRuntimeGameClientEvent(kind, clientPayload);
        if (kind === 'notification') {
            useNotificationStore.getState().pushNotification(clientPayload);
        }
        return;
    }

    if (name === 'runtimeWorkerError') {
        console.warn('Backend worker error:', payload);
        return;
    }

    if (name === 'updateIsGameRunning') {
        if (!isHostCapabilityAvailable('gameProcessMonitor')) {
            return;
        }
        handleGameRunningUpdate(payload).catch((error: unknown) => {
            useNotificationStore.getState().pushNotification({
                level: 'warning',
                title: 'Game state update failed',
                message: error instanceof Error ? error.message : String(error)
            });
        });
        return;
    }

    if (name === 'browserFocus') {
        runtimeStore.setGameState({
            lastBrowserFocusAt: new Date().toISOString()
        });
        requestGameRunningStateRefresh('browser focus');
        handleBrowserFocus().catch((error: unknown) => {
            console.warn('Browser focus status refresh failed:', error);
        });
    }
}

export async function bindRuntimeEvents(): Promise<() => void> {
    resetFriendProfileProjectionBatch();
    const unsubscribers: RuntimeEventUnsubscribe[] = [];
    const events: RuntimeEventName[] = [
        'addGameLogEvent',
        'backendRuntimeTelemetry',
        'gameLogProjection',
        'gameLogPersistenceFallback',
        'gameLogSideEffect',
        'runtimeGroupInstancesProjection',
        'overlayActivitySnapshot',
        'printsAutoCleanup',
        'favoritesChanged',
        'friendProfileLoadStatus',
        'gameClientEvent',
        'runtimeWorkerError',
        'realtimeFriendProjection',
        'realtimeUserProjection',
        'realtimeEntryCorrection',
        'realtimeNotificationProjection',
        'realtimeCurrentUserProjection',
        'realtimeInstanceClosedProjection',
        'realtimeInstanceQueueProjection',
        'updateIsGameRunning',
        'browserFocus'
    ];

    useSessionStore.getState().setTransportStatus('runtime-subscribing');

    try {
        for (const name of events) {
            const unsubscribe = await subscribeRuntimeEvent(name);
            unsubscribers.push(unsubscribe);
        }
    } catch (error) {
        unsubscribeRuntimeEvents(unsubscribers);
        useRuntimeStore.getState().setShellState({
            backendRuntimeSnapshotHydrated: true,
            backendRuntimeSessionHydrating: false
        });
        useSessionStore.getState().setTransportStatus('disconnected');
        throw error;
    }

    useSessionStore.getState().setTransportStatus('runtime-subscribed');
    try {
        const snapshot = await commands.appGetBackendRuntimeSnapshot();
        await hydrateBackendRuntimeSnapshot(snapshot);
    } catch (error) {
        useRuntimeStore.getState().setShellState({
            backendRuntimeSnapshotHydrated: true,
            backendRuntimeSessionHydrating: false
        });
        console.warn('Failed to hydrate backend runtime snapshot:', error);
    }
    try {
        unsubscribers.push(await bindDeepLinkEvents());
    } catch (error) {
        unsubscribeRuntimeEvents(unsubscribers);
        useSessionStore.getState().setTransportStatus('disconnected');
        throw error;
    }
    await drainPendingDeepLinks();
    requestGroupInstancesRefresh(
        'runtime event binding after backend snapshot hydration'
    );

    return () => {
        resetFriendProfileProjectionBatch();
        unsubscribeRuntimeEvents(unsubscribers);
        useSessionStore.getState().setTransportStatus('disconnected');
    };
}

function unsubscribeRuntimeEvents(
    unsubscribers: RuntimeEventUnsubscribe[]
): void {
    for (const unsubscribe of unsubscribers) {
        if (typeof unsubscribe === 'function') {
            unsubscribe();
        }
    }
}

function subscribeRuntimeEvent<Name extends RuntimeEventName>(
    name: Name
): Promise<RuntimeEventUnsubscribe> {
    return tauriClient.events.subscribe<RuntimeEventPayloadMap[Name]>(
        name,
        (payload) => {
            handleRuntimeEvent(name, payload);
        }
    );
}
