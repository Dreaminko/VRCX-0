import gameLogPersistenceRepository from '@/repositories/gameLogPersistenceRepository';
import { useInstanceJoinHistoryStore } from '@/state/instanceJoinHistoryStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';

import {
    ensureCurrentAuthAttempt,
    isAuthAttemptSupersededError,
    type AuthAttempt
} from './authAttempt';
import { restoreRuntimeGameLogProjectionFromPersistence } from './gameLogIngestService';
import { requestGroupInstancesRefresh } from './runtime-event-bridge/auxiliaryEventHandlers';
import { syncStartupServicesTask } from './startupServicesStatus';

type AuthenticatedUser = Record<string, unknown> & {
    id?: unknown;
    displayName?: unknown;
    username?: unknown;
};

function getCurrentUserDisplayName(
    user: AuthenticatedUser | null | undefined
): string {
    return String(user?.displayName || user?.username || user?.id || '');
}

async function loadInstanceJoinHistory(
    userId: string,
    attempt: AuthAttempt
): Promise<void> {
    let history: Iterable<[unknown, unknown]>;
    try {
        history =
            await gameLogPersistenceRepository.getInstanceJoinHistory(userId);
    } catch (error) {
        console.warn(
            'Instance join history is unavailable during session bootstrap:',
            error
        );
        return;
    }

    ensureCurrentAuthAttempt(attempt);
    useInstanceJoinHistoryStore.getState().setInstanceJoinHistory(history);
}

async function hydratePostReadySession(
    userId: string,
    attempt: AuthAttempt
): Promise<void> {
    await requestGroupInstancesRefresh('session bootstrap');
    ensureCurrentAuthAttempt(attempt);
    await loadInstanceJoinHistory(userId, attempt);
    await restoreRuntimeGameLogProjectionFromPersistence().catch(
        (error: unknown) => {
            console.warn(
                'Current GameLog roster restore failed during session bootstrap:',
                error
            );
        }
    );
}

function startPostReadySessionHydration(
    userId: string,
    attempt: AuthAttempt
): void {
    void hydratePostReadySession(userId, attempt).catch((error: unknown) => {
        if (!isAuthAttemptSupersededError(error)) {
            console.warn('Post-ready session hydration failed:', error);
        }
    });
}

export async function bootstrapAuthenticatedSession(
    user: AuthenticatedUser | null | undefined,
    attempt: AuthAttempt
): Promise<void> {
    const userId =
        typeof user?.id === 'string'
            ? user.id.trim()
            : String(user?.id ?? '').trim();
    if (!userId) {
        throw new Error('Session bootstrap requires an authenticated user id.');
    }

    const displayName = getCurrentUserDisplayName(user) || userId;
    const runtimeStore = useRuntimeStore.getState();
    const sessionStore = useSessionStore.getState();

    ensureCurrentAuthAttempt(attempt);
    sessionStore.setSessionState({
        isLoggedIn: false,
        isFriendsLoaded: false,
        isFavoritesLoaded: false,
        sessionPhase: 'bootstrapping'
    });
    runtimeStore.setStartupTask(
        'services',
        'running',
        `Preparing the interface for ${displayName}.`
    );

    sessionStore.setSessionState({
        isLoggedIn: true,
        isFriendsLoaded: false,
        isFavoritesLoaded: false,
        sessionPhase: 'ready'
    });
    syncStartupServicesTask([
        `Authenticated session is ready for ${displayName}.`
    ]);
    startPostReadySessionHydration(userId, attempt);
}
