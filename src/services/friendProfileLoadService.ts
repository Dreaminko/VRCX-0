import userProfileRepository from '@/repositories/userProfileRepository';
import {
    executeWithBackoff,
    isBackoffCancelledError
} from '@/shared/utils/retry';
import { useFriendRosterStore } from '@/state/friendRosterStore';
import {
    type FriendProfileLoadState,
    useRuntimeStore
} from '@/state/runtimeStore';

const FRIEND_PROFILE_LOAD_CONCURRENCY = 3;
const FRIEND_PROFILE_LOAD_MAX_RETRIES = 4;
const FRIEND_PROFILE_LOAD_BASE_DELAY_MS = 500;
const TERMINAL_RESET_DELAY_MS = 5000;
const ACTIVE_STATUSES = new Set<FriendProfileLoadState['status']>([
    'running',
    'cancelling'
]);

type StartFriendProfileLoadInput = {
    ownerUserId: string;
    endpoint?: string;
    friendIds: string[];
};

type FriendProfileLoadTerminalStatus = Extract<
    FriendProfileLoadState['status'],
    'completed' | 'cancelled' | 'error'
>;

let nextRunId = 1;
let resetTimer: ReturnType<typeof setTimeout> | null = null;

function normalizeString(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function isRateLimitedError(error: unknown): boolean {
    return (
        (isRecord(error) && error.status === 429) ||
        (error instanceof Error && error.message.includes('429'))
    );
}

function clearResetTimer(): void {
    if (resetTimer !== null) {
        clearTimeout(resetTimer);
        resetTimer = null;
    }
}

function isCurrentRun(
    runId: number,
    ownerUserId: string,
    endpoint: string
): boolean {
    const runtime = useRuntimeStore.getState();
    return (
        runtime.friendProfileLoad.runId === runId &&
        runtime.friendProfileLoad.ownerUserId === ownerUserId &&
        runtime.friendProfileLoad.ownerEndpoint === endpoint &&
        runtime.auth.currentUserId === ownerUserId &&
        runtime.auth.currentUserEndpoint === endpoint
    );
}

function isRunCancelled(
    runId: number,
    ownerUserId: string,
    endpoint: string
): boolean {
    if (!isCurrentRun(runId, ownerUserId, endpoint)) {
        return true;
    }
    const state = useRuntimeStore.getState().friendProfileLoad;
    return state.cancelRequested || state.status === 'cancelling';
}

function scheduleTerminalReset(runId: number): void {
    clearResetTimer();
    resetTimer = setTimeout(() => {
        resetTimer = null;
        if (useRuntimeStore.getState().friendProfileLoad.runId === runId) {
            useRuntimeStore.getState().resetFriendProfileLoadState();
        }
    }, TERMINAL_RESET_DELAY_MS);
}

function finishRun(
    runId: number,
    ownerUserId: string,
    endpoint: string,
    status: FriendProfileLoadTerminalStatus,
    lastError: string | null = null
): void {
    if (!isCurrentRun(runId, ownerUserId, endpoint)) {
        return;
    }
    useRuntimeStore.getState().setFriendProfileLoadState({
        status,
        dialogOpen: false,
        finishedAt: new Date().toISOString(),
        lastError
    });
    scheduleTerminalReset(runId);
}

async function runFriendProfileLoad(
    runId: number,
    ownerUserId: string,
    endpoint: string,
    friendIds: string[]
): Promise<void> {
    let nextFriendIndex = 0;

    async function loadNextFriendProfile(): Promise<void> {
        while (!isRunCancelled(runId, ownerUserId, endpoint)) {
            const friendId = friendIds[nextFriendIndex];
            nextFriendIndex += 1;
            if (!friendId) {
                return;
            }

            let loaded = false;
            let failed = false;
            try {
                const profile = await executeWithBackoff(
                    () =>
                        userProfileRepository.getUserProfile({
                            userId: friendId,
                            endpoint
                        }),
                    {
                        maxRetries: FRIEND_PROFILE_LOAD_MAX_RETRIES,
                        baseDelay: FRIEND_PROFILE_LOAD_BASE_DELAY_MS,
                        shouldRetry: isRateLimitedError,
                        isCancelled: () =>
                            isRunCancelled(runId, ownerUserId, endpoint)
                    }
                );
                if (isRunCancelled(runId, ownerUserId, endpoint)) {
                    return;
                }
                if (!profile?.id) {
                    failed = true;
                    continue;
                }
                const friendRoster = useFriendRosterStore.getState();
                if (!friendRoster.friendsById[friendId]) {
                    continue;
                }
                friendRoster.applyFriendPatch({
                    userId: friendId,
                    patch: profile,
                    stateBucketAuthority: 'preserve'
                });
                loaded = true;
            } catch (error) {
                if (
                    isBackoffCancelledError(error) ||
                    isRunCancelled(runId, ownerUserId, endpoint)
                ) {
                    return;
                }
                failed = true;
                console.warn(
                    '[FriendProfileLoadService] Failed to load friend profile',
                    friendId,
                    error
                );
            } finally {
                if (isCurrentRun(runId, ownerUserId, endpoint)) {
                    const runtime = useRuntimeStore.getState();
                    const current = runtime.friendProfileLoad;
                    runtime.setFriendProfileLoadState({
                        processedFriends: Math.min(
                            current.totalFriends,
                            current.processedFriends + 1
                        ),
                        loadedFriends: current.loadedFriends + (loaded ? 1 : 0),
                        failedFriends: current.failedFriends + (failed ? 1 : 0)
                    });
                }
            }
        }
    }

    const workerCount = Math.min(
        FRIEND_PROFILE_LOAD_CONCURRENCY,
        friendIds.length
    );
    await Promise.all(
        Array.from({ length: workerCount }, () => loadNextFriendProfile())
    );
    if (!isCurrentRun(runId, ownerUserId, endpoint)) {
        return;
    }
    finishRun(
        runId,
        ownerUserId,
        endpoint,
        isRunCancelled(runId, ownerUserId, endpoint) ? 'cancelled' : 'completed'
    );
}

export function startFriendProfileLoad({
    ownerUserId,
    endpoint = '',
    friendIds
}: StartFriendProfileLoadInput): number {
    const current = useRuntimeStore.getState().friendProfileLoad;
    if (ACTIVE_STATUSES.has(current.status)) {
        openFriendProfileLoadDialog();
        return current.runId;
    }

    const normalizedOwnerUserId = normalizeString(ownerUserId);
    const normalizedEndpoint = normalizeString(endpoint);
    const runtimeAuth = useRuntimeStore.getState().auth;
    if (
        !normalizedOwnerUserId ||
        runtimeAuth.currentUserId !== normalizedOwnerUserId ||
        runtimeAuth.currentUserEndpoint !== normalizedEndpoint
    ) {
        throw new Error('Friend profile loading requires the active account.');
    }

    const normalizedFriendIds = Array.from(
        new Set(friendIds.map(normalizeString).filter(Boolean))
    );
    if (!normalizedFriendIds.length) {
        throw new Error('Friend profile loading requires at least one friend.');
    }

    clearResetTimer();
    const runId = nextRunId++;
    const startedAt = new Date().toISOString();
    useRuntimeStore.getState().setFriendProfileLoadState({
        runId,
        status: 'running',
        ownerUserId: normalizedOwnerUserId,
        ownerEndpoint: normalizedEndpoint,
        totalFriends: normalizedFriendIds.length,
        processedFriends: 0,
        loadedFriends: 0,
        failedFriends: 0,
        cancelRequested: false,
        dialogOpen: true,
        startedAt,
        updatedAt: startedAt,
        finishedAt: null,
        lastError: null
    });

    void runFriendProfileLoad(
        runId,
        normalizedOwnerUserId,
        normalizedEndpoint,
        normalizedFriendIds
    ).catch((error: unknown) => {
        finishRun(
            runId,
            normalizedOwnerUserId,
            normalizedEndpoint,
            'error',
            error instanceof Error
                ? error.message
                : 'Failed to load friend details.'
        );
    });
    return runId;
}

function setActiveFriendProfileLoadState(
    patch: Partial<FriendProfileLoadState>
): void {
    const runtime = useRuntimeStore.getState();
    if (!ACTIVE_STATUSES.has(runtime.friendProfileLoad.status)) {
        return;
    }
    runtime.setFriendProfileLoadState(patch);
}

export function cancelFriendProfileLoad(): void {
    setActiveFriendProfileLoadState({
        status: 'cancelling',
        cancelRequested: true
    });
}

export function minimizeFriendProfileLoadDialog(): void {
    setActiveFriendProfileLoadState({ dialogOpen: false });
}

export function openFriendProfileLoadDialog(): void {
    setActiveFriendProfileLoadState({ dialogOpen: true });
}

export function resetFriendProfileLoadService(): void {
    clearResetTimer();
    useRuntimeStore.getState().resetFriendProfileLoadState();
}
