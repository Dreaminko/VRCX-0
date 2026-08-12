import type {
    AuthenticatedSessionProjection,
    FriendProfileLoadStatusPayload
} from '@/platform/tauri/bindings';
import { useRuntimeStore } from '@/state/runtimeStore';

import { applyAuthenticatedSessionProjection } from '../backendRuntimeSessionResumeService';
import { applyFriendProfileLoadStatusPayload } from '../friendProfileLoadService';
import { isRecord } from './guards';
import type { RuntimeSnapshotPayload } from './types';

let backendRuntimeHydrationPromise: Promise<void> | null = null;
let pendingBackendRuntimeHydrationSnapshot: RuntimeSnapshotPayload = null;
let pendingAuthenticatedSessionProjection: AuthenticatedSessionProjection = {
    revision: 0,
    session: null
};
let hasPendingBackendRuntimeHydrationSnapshot = false;

function applyBackendRuntimeSnapshot(
    snapshot: RuntimeSnapshotPayload,
    {
        markHydrated = true,
        applyFriendProfileLoad = false
    }: { markHydrated?: boolean; applyFriendProfileLoad?: boolean } = {}
): void {
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

export function hydrateBackendRuntimeSnapshot(
    snapshot: RuntimeSnapshotPayload,
    authenticatedSession: AuthenticatedSessionProjection,
    reconcilePendingProjectionEvents: () => void
): Promise<void> {
    pendingBackendRuntimeHydrationSnapshot = snapshot;
    pendingAuthenticatedSessionProjection = authenticatedSession;
    hasPendingBackendRuntimeHydrationSnapshot = true;

    if (!backendRuntimeHydrationPromise) {
        useRuntimeStore.getState().setShellState({
            backendRuntimeSessionHydrating: true
        });
        backendRuntimeHydrationPromise = (async () => {
            while (hasPendingBackendRuntimeHydrationSnapshot) {
                const nextSnapshot = pendingBackendRuntimeHydrationSnapshot;
                const nextAuthenticatedSession =
                    pendingAuthenticatedSessionProjection;
                pendingBackendRuntimeHydrationSnapshot = null;
                hasPendingBackendRuntimeHydrationSnapshot = false;
                applyBackendRuntimeSnapshot(nextSnapshot, {
                    markHydrated: false,
                    applyFriendProfileLoad: true
                });
                try {
                    await applyAuthenticatedSessionProjection(
                        nextAuthenticatedSession
                    );
                    reconcilePendingProjectionEvents();
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

export function handleBackendRuntimeSyncSnapshot(
    snapshot: RuntimeSnapshotPayload,
    reconcilePendingProjectionEvents: () => void
): void {
    if (!useRuntimeStore.getState().shell.backendRuntimeSnapshotHydrated) {
        hydrateBackendRuntimeSnapshot(
            snapshot,
            useRuntimeStore.getState().authenticatedSession,
            reconcilePendingProjectionEvents
        );
        return;
    }

    applyBackendRuntimeSnapshot(snapshot);
    applyAuthenticatedSessionProjection(
        useRuntimeStore.getState().authenticatedSession
    )
        .catch((error: unknown) => {
            console.warn(
                'Failed to resume frontend session from backend runtime:',
                error
            );
        })
        .then(() => {
            reconcilePendingProjectionEvents();
        });
}

export function handleAuthenticatedSessionProjection(
    projection: AuthenticatedSessionProjection,
    reconcilePendingProjectionEvents: () => void
): void {
    applyAuthenticatedSessionProjection(projection)
        .catch((error: unknown) => {
            console.warn(
                'Failed to apply authenticated session projection:',
                error
            );
        })
        .then(() => {
            reconcilePendingProjectionEvents();
        });
}
