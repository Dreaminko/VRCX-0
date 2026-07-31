import { commands } from '@/platform/tauri/bindings';
import { useFavoriteStore } from '@/state/favoriteStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';

import { refreshModerationSync } from './moderationSyncService';

type RuntimeAuthSnapshot = {
    currentUserId: string | null;
    currentUserEndpoint: string;
    currentUserWebsocket: string;
    currentUserSnapshot: Record<string, unknown> | null;
};

type RuntimeAuthTarget = {
    currentUserId: string;
    currentUserEndpoint: string;
    currentUserWebsocket: string;
};

type RefreshCurrentUserOptions = {
    expectedUserId?: unknown;
    expectedEndpoint?: unknown;
    expectedWebsocket?: unknown;
    overlayPatch?: unknown;
};

type RefreshPlayerModerationsOptions = {
    isCurrent?: (() => boolean) | null;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function getRuntimeAuth(): RuntimeAuthSnapshot {
    const runtimeState = useRuntimeStore.getState();
    return {
        currentUserId: runtimeState.auth.currentUserId,
        currentUserEndpoint: runtimeState.auth.currentUserEndpoint,
        currentUserWebsocket: runtimeState.auth.currentUserWebsocket,
        currentUserSnapshot: isRecord(runtimeState.auth.currentUserSnapshot)
            ? runtimeState.auth.currentUserSnapshot
            : null
    };
}

function normalizeRuntimeAuthValue(value: unknown) {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

export async function refreshCurrentUser({
    expectedUserId = '',
    expectedEndpoint = '',
    expectedWebsocket = '',
    overlayPatch = null
}: RefreshCurrentUserOptions = {}): Promise<boolean | null> {
    void overlayPatch;
    const auth = getRuntimeAuth();
    const target: RuntimeAuthTarget = {
        currentUserId: normalizeRuntimeAuthValue(
            expectedUserId || auth.currentUserId
        ),
        currentUserEndpoint: normalizeRuntimeAuthValue(
            expectedEndpoint || auth.currentUserEndpoint
        ),
        currentUserWebsocket: normalizeRuntimeAuthValue(
            expectedWebsocket || auth.currentUserWebsocket
        )
    };

    if (!target.currentUserId) {
        return null;
    }
    if (
        target.currentUserId !==
            normalizeRuntimeAuthValue(auth.currentUserId) ||
        target.currentUserEndpoint !==
            normalizeRuntimeAuthValue(auth.currentUserEndpoint) ||
        target.currentUserWebsocket !==
            normalizeRuntimeAuthValue(auth.currentUserWebsocket)
    ) {
        return null;
    }

    const result = await commands.appCurrentUserRefresh();
    return result.applied;
}

export async function refreshFriendAndFavoriteSnapshots(
    _options: { syncRealtime?: boolean } = {}
) {
    void _options;
    const auth = getRuntimeAuth();
    if (!auth.currentUserId || !auth.currentUserSnapshot) {
        return;
    }

    const result = await commands.appSocialBaselineRefresh();
    const favoritesSnapshot = isRecord(result.favoritesSnapshot)
        ? result.favoritesSnapshot
        : null;
    if (!favoritesSnapshot) {
        return;
    }

    const latestAuth = getRuntimeAuth();
    const sessionState = useSessionStore.getState();
    if (
        latestAuth.currentUserId !== auth.currentUserId ||
        latestAuth.currentUserEndpoint !== auth.currentUserEndpoint ||
        !sessionState.isLoggedIn ||
        sessionState.sessionPhase !== 'ready'
    ) {
        return;
    }
    useFavoriteStore.getState().setFavoritesSnapshot({
        ...favoritesSnapshot,
        detail: String(favoritesSnapshot.detail || '')
    });
    useSessionStore.getState().setFavoritesLoaded(true);
}

export async function refreshPlayerModerations({
    isCurrent = null
}: RefreshPlayerModerationsOptions = {}) {
    const { currentUserId, currentUserEndpoint } = getRuntimeAuth();
    if (!currentUserId) {
        return;
    }

    await refreshModerationSync({
        userId: currentUserId,
        endpoint: currentUserEndpoint
    });

    const latestAuth = getRuntimeAuth();
    if (
        latestAuth.currentUserId !== currentUserId ||
        latestAuth.currentUserEndpoint !== currentUserEndpoint ||
        (typeof isCurrent === 'function' && !isCurrent())
    ) {
        return;
    }
}
