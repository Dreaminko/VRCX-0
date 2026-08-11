import configRepository from '@/repositories/configRepository';
import type {
    RealtimeCurrentUserProjectionPayload,
    RealtimeEntryCorrectionPayload,
    RealtimeFeedProjectionPayload,
    RealtimeFriendProjectionPayload,
    RealtimeInstanceClosedProjectionPayload,
    RealtimeNotificationProjectionPayload,
    RealtimeUserProjectionPayload
} from '@/services/runtime-event-bridge/realtimeProjectionTypes';
import { useFeedLiveStore } from '@/state/feedLiveStore';
import { useFriendLogStore } from '@/state/friendLogStore';
import { useFriendRosterStore } from '@/state/friendRosterStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useShellStore } from '@/state/shellStore';
import { useUserFactsStore } from '@/state/userFactsStore';
import { useVrcNotificationStore } from '@/state/vrcNotificationStore';

import { buildAvatarWearSnapshotUpdate } from './avatarWearTimeService';
import { recordCurrentUserSnapshot } from './domainIngestionService';
import { handleRealtimeInstanceQueueProjection } from './realtimeInstanceQueueService';
import { pushSharedFeedNotification } from './sharedFeedNotificationService';

type ProjectionRecord = Record<string, unknown>;
type RuntimeState = ReturnType<typeof useRuntimeStore.getState>;
const CURRENT_USER_FRIEND_ARRAY_FIELDS = [
    'friends',
    'onlineFriends',
    'activeFriends',
    'offlineFriends'
];

function isRecord(value: unknown): value is ProjectionRecord {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function hasOwn(record: ProjectionRecord, key: string): boolean {
    return Object.prototype.hasOwnProperty.call(record, key);
}

function normalizeUserId(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function trimCorrectionId(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function getCurrentUserSnapshot(
    runtimeState: RuntimeState = useRuntimeStore.getState()
) {
    return isRecord(runtimeState.auth.currentUserSnapshot)
        ? runtimeState.auth.currentUserSnapshot
        : null;
}

function currentUserDisplayName(
    snapshot: ProjectionRecord,
    fallback: unknown = ''
) {
    return (
        normalizeUserId(snapshot.displayName) ||
        normalizeUserId(snapshot.username) ||
        normalizeUserId(snapshot.id) ||
        normalizeUserId(fallback)
    );
}

function hasCompleteCurrentUserFriendBucketSnapshot(source: ProjectionRecord) {
    return CURRENT_USER_FRIEND_ARRAY_FIELDS.every((field) =>
        Array.isArray(source[field])
    );
}

function getCurrentUserProjectionFriendBucketSource(
    payload: RealtimeCurrentUserProjectionPayload
) {
    const patch = payload.patch;
    if (hasCompleteCurrentUserFriendBucketSnapshot(patch)) {
        return patch;
    }
    const snapshot = payload.snapshot;
    if (
        Object.keys(patch).length === 0 &&
        hasCompleteCurrentUserFriendBucketSnapshot(snapshot)
    ) {
        return snapshot;
    }
    return null;
}

function mergeCurrentUserProjectionSnapshot(
    runtimeState: RuntimeState,
    payload: RealtimeCurrentUserProjectionPayload
) {
    const currentSnapshot = getCurrentUserSnapshot(runtimeState);
    const source = Object.keys(payload.patch).length
        ? payload.patch
        : payload.snapshot;
    const completeFriendBucketSource =
        getCurrentUserProjectionFriendBucketSource(payload);
    const nextSnapshot: ProjectionRecord = {
        ...(currentSnapshot || {}),
        ...source
    };

    if (completeFriendBucketSource) {
        for (const field of CURRENT_USER_FRIEND_ARRAY_FIELDS) {
            nextSnapshot[field] = completeFriendBucketSource[field];
        }
    }

    if (currentSnapshot) {
        for (const field of CURRENT_USER_FRIEND_ARRAY_FIELDS) {
            if (
                !completeFriendBucketSource &&
                Array.isArray(currentSnapshot[field])
            ) {
                nextSnapshot[field] = currentSnapshot[field];
            }
        }
    }

    return nextSnapshot;
}

function handleRealtimeFeedProjection(payload: RealtimeFeedProjectionPayload) {
    const currentUserId = useRuntimeStore.getState().auth.currentUserId ?? '';
    if (payload.ownerUserId !== currentUserId) {
        return;
    }
    const upserts = (payload.upserts ?? []).filter(
        (upsert) => Object.keys(upsert.entry).length > 0
    );
    useFeedLiveStore.getState().pushEntries(upserts, {
        ownerUserId: payload.ownerUserId
    });
    useFeedLiveStore.getState().pushPatches(payload.patches ?? []);
    for (const upsert of upserts) {
        pushSharedFeedNotification(upsert.entry).catch((error: unknown) => {
            console.warn(
                'Failed to publish realtime feed notification:',
                error
            );
        });
    }
}

function clearNotificationMenuIfNoUnseen() {
    if (useVrcNotificationStore.getState().unseenCount === 0) {
        useShellStore.getState().removeNotify('notification');
    }
}

function notifyNotificationMenu(notification: ProjectionRecord) {
    if (notification.version === 2 && notification.seen !== false) {
        return;
    }
    useShellStore.getState().notifyMenu('notification');
}

function parseStringArray(value: unknown): string[] {
    if (Array.isArray(value)) {
        return value.map((entry) => normalizeUserId(entry)).filter(Boolean);
    }
    if (typeof value !== 'string') {
        return [];
    }
    try {
        const parsed = JSON.parse(value);
        return Array.isArray(parsed)
            ? parsed.map((entry) => normalizeUserId(entry)).filter(Boolean)
            : [];
    } catch {
        return [];
    }
}

async function shouldNotifyInstanceClosed(): Promise<boolean> {
    try {
        const filters = parseStringArray(
            await configRepository.getString(
                'VRCX_notificationTableFilters',
                '[]'
            )
        );
        return !filters.length || filters.includes('instance.closed');
    } catch {
        return true;
    }
}

function handleRealtimeFriendProjection(
    payload: RealtimeFriendProjectionPayload
) {
    for (const userId of payload.removals ?? []) {
        const normalizedUserId = normalizeUserId(userId);
        if (!normalizedUserId) {
            continue;
        }
        useFriendRosterStore.getState().removeFriend(normalizedUserId);
    }

    const patchEntries = (payload.patches ?? []).map((patchEntry) => {
        const patch = patchEntry.patch;
        return {
            userId: normalizeUserId(
                patchEntry.userId || patch.id || patch.userId
            ),
            patch,
            stateBucket: normalizeUserId(
                patchEntry.stateBucket || patch.stateBucket || patch.state
            ),
            stateBucketAuthority: normalizeUserId(
                patchEntry.stateBucketAuthority || 'explicit'
            )
        };
    });
    if (patchEntries.length) {
        useFriendRosterStore.getState().applyFriendPatches(patchEntries);
    }

    if (payload.friendLogChanged) {
        useShellStore.getState().notifyMenu('friend-log');
        useFriendLogStore.getState().bumpRevision();
    }
}

export function handleRealtimeUserCacheProjection(
    payload: RealtimeUserProjectionPayload
) {
    useUserFactsStore.getState().replaceUserFacts(payload.users);
}

async function handleRealtimeNotificationProjection(
    payload: RealtimeNotificationProjectionPayload
) {
    const store = useVrcNotificationStore.getState();

    if (payload.expiredIds?.length) {
        store.expireNotifications(payload.expiredIds);
    }
    if (payload.seenIds?.length) {
        store.markNotificationsSeen(payload.seenIds);
    }

    for (const upsert of payload.upserts ?? []) {
        let notification = upsert.notification;
        if (!notification.id) {
            continue;
        }
        const existingNotification = store.rows.find(
            (row) => row.id === notification.id
        );
        const insertDefaults = upsert.insertDefaults;
        if (
            !existingNotification &&
            insertDefaults &&
            Object.keys(insertDefaults).length
        ) {
            notification = {
                ...insertDefaults,
                ...notification
            };
        }
        store.upsertNotification(notification);
        const mergedNotification =
            useVrcNotificationStore
                .getState()
                .rows.find((row) => row.id === notification.id) || notification;
        if (upsert.notifyMenu) {
            notifyNotificationMenu(mergedNotification);
        }
    }

    if (payload.clearMenuIfNoUnseen) {
        clearNotificationMenuIfNoUnseen();
    }
}

function handleRealtimeEntryCorrection(
    payload: RealtimeEntryCorrectionPayload
) {
    const id = trimCorrectionId(payload.id);
    if (!id || !Object.keys(payload.fields).length) {
        return;
    }
    if (payload.stream === 'notification') {
        useVrcNotificationStore
            .getState()
            .patchNotification(id, payload.fields);
    }
}

function handleRealtimeCurrentUserProjection(
    payload: RealtimeCurrentUserProjectionPayload
) {
    const runtimeStore = useRuntimeStore.getState();
    const mergedSnapshot = mergeCurrentUserProjectionSnapshot(
        runtimeStore,
        payload
    );
    const { snapshot: stampedSnapshot } = buildAvatarWearSnapshotUpdate({
        previousSnapshot: runtimeStore.auth.currentUserSnapshot,
        nextSnapshot: mergedSnapshot,
        isGameRunning: runtimeStore.gameState.isGameRunning
    });
    const snapshot = isRecord(stampedSnapshot)
        ? stampedSnapshot
        : mergedSnapshot;
    runtimeStore.setAuthBootstrap({
        currentUserSnapshot: snapshot,
        currentUserDisplayName: currentUserDisplayName(
            snapshot,
            runtimeStore.auth.currentUserDisplayName
        )
    });
    const patch = payload.patch;
    if (hasOwn(patch, 'queuedInstance')) {
        const queuedInstance = normalizeUserId(patch.queuedInstance);
        if (queuedInstance) {
            handleRealtimeInstanceQueueProjection({
                kind: 'update',
                instanceLocation: queuedInstance
            });
        } else if (useRuntimeStore.getState().instanceQueue.active) {
            useRuntimeStore.getState().clearInstanceQueueState();
        }
    }
    if (payload.gameStatePatch) {
        runtimeStore.setGameState(payload.gameStatePatch);
    }
    recordCurrentUserSnapshot(snapshot, {
        endpoint: runtimeStore.auth.currentUserEndpoint,
        source: 'currentUser'
    });
}

async function handleRealtimeInstanceClosedProjection(
    payload: RealtimeInstanceClosedProjectionPayload
) {
    const notification = payload.notification;
    if (!notification.id) {
        return;
    }
    useVrcNotificationStore.getState().upsertNotification(notification);
    if (await shouldNotifyInstanceClosed()) {
        useShellStore.getState().notifyMenu('notification');
    }
}

export {
    handleRealtimeCurrentUserProjection,
    handleRealtimeEntryCorrection,
    handleRealtimeFeedProjection,
    handleRealtimeFriendProjection,
    handleRealtimeInstanceClosedProjection,
    handleRealtimeNotificationProjection
};
