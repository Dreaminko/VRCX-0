import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useFriendRosterStore } from '@/state/friendRosterStore';
import { useRuntimeStore } from '@/state/runtimeStore';

const mocks = vi.hoisted(() => ({
    getUserProfile: vi.fn()
}));

vi.mock('@/repositories/userProfileRepository', () => ({
    default: {
        getUserProfile: mocks.getUserProfile
    }
}));

import {
    cancelFriendProfileLoad,
    minimizeFriendProfileLoadDialog,
    resetFriendProfileLoadService,
    startFriendProfileLoad
} from './friendProfileLoadService';

const OWNER_USER_ID = 'usr_owner';
const ENDPOINT = 'https://api.example.test';

function createDeferred<T>() {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>((nextResolve) => {
        resolve = nextResolve;
    });
    return { promise, resolve };
}

function seedFriend(userId: string, stateBucket = 'online') {
    useFriendRosterStore.getState().applyFriendPatch({
        userId,
        patch: {
            id: userId,
            displayName: userId
        },
        stateBucket
    });
}

async function waitForStatus(status: string) {
    await vi.waitFor(() => {
        expect(useRuntimeStore.getState().friendProfileLoad.status).toBe(
            status
        );
    });
}

describe('friendProfileLoadService', () => {
    beforeEach(() => {
        resetFriendProfileLoadService();
        useRuntimeStore.getState().resetRuntimeState();
        useFriendRosterStore.getState().resetRoster();
        useRuntimeStore.getState().setAuthBootstrap({
            currentUserId: OWNER_USER_ID,
            currentUserEndpoint: ENDPOINT
        });
        mocks.getUserProfile.mockReset();
        vi.useRealTimers();
    });

    it('limits requests to three workers and records completed progress', async () => {
        let activeRequests = 0;
        let maxActiveRequests = 0;
        const pending: Array<() => void> = [];
        mocks.getUserProfile.mockImplementation(
            ({ userId }: { userId: string }) =>
                new Promise((resolve) => {
                    activeRequests += 1;
                    maxActiveRequests = Math.max(
                        maxActiveRequests,
                        activeRequests
                    );
                    pending.push(() => {
                        activeRequests -= 1;
                        resolve({ id: userId, date_joined: '2026-01-01' });
                    });
                })
        );

        const friendIds = Array.from({ length: 5 }, (_, index) => {
            const userId = `usr_${index}`;
            seedFriend(userId);
            return userId;
        });
        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds
        });

        await vi.waitFor(() => {
            expect(mocks.getUserProfile).toHaveBeenCalledTimes(3);
        });
        pending.splice(0).forEach((resolve) => resolve());
        await vi.waitFor(() => {
            expect(mocks.getUserProfile).toHaveBeenCalledTimes(5);
        });
        pending.splice(0).forEach((resolve) => resolve());
        await waitForStatus('completed');

        expect(maxActiveRequests).toBe(3);
        expect(useRuntimeStore.getState().friendProfileLoad).toMatchObject({
            processedFriends: 5,
            loadedFriends: 5,
            failedFriends: 0,
            totalFriends: 5,
            dialogOpen: false
        });
    });

    it('includes already-loaded friends in full-roster progress', async () => {
        const deferred = createDeferred<Record<string, unknown>>();
        mocks.getUserProfile.mockReturnValue(deferred.promise);
        seedFriend('usr_missing');

        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_missing'],
            totalFriendCount: 3
        });
        expect(useRuntimeStore.getState().friendProfileLoad).toMatchObject({
            status: 'running',
            processedFriends: 2,
            totalFriends: 3
        });

        deferred.resolve({
            id: 'usr_missing',
            date_joined: '2026-01-01'
        });
        await waitForStatus('completed');
        expect(useRuntimeStore.getState().friendProfileLoad).toMatchObject({
            processedFriends: 3,
            totalFriends: 3,
            loadedFriends: 1
        });
    });

    it('retries rate limits and counts non-retryable failures', async () => {
        vi.useFakeTimers();
        seedFriend('usr_retry');
        mocks.getUserProfile
            .mockRejectedValueOnce({ status: 429 })
            .mockResolvedValueOnce({
                id: 'usr_retry',
                date_joined: '2026-01-01'
            });

        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_retry']
        });
        await Promise.resolve();
        expect(mocks.getUserProfile).toHaveBeenCalledTimes(1);

        await vi.advanceTimersByTimeAsync(500);
        expect(mocks.getUserProfile).toHaveBeenCalledTimes(2);
        expect(useRuntimeStore.getState().friendProfileLoad).toMatchObject({
            status: 'completed',
            loadedFriends: 1,
            failedFriends: 0
        });

        resetFriendProfileLoadService();
        vi.useRealTimers();
        seedFriend('usr_failed');
        mocks.getUserProfile.mockReset();
        mocks.getUserProfile.mockRejectedValueOnce(new Error('failed'));
        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_failed']
        });
        await waitForStatus('completed');
        expect(useRuntimeStore.getState().friendProfileLoad).toMatchObject({
            processedFriends: 1,
            loadedFriends: 0,
            failedFriends: 1
        });
    });

    it('preserves the latest live friend bucket when a profile arrives', async () => {
        const deferred = createDeferred<Record<string, unknown>>();
        mocks.getUserProfile.mockReturnValue(deferred.promise);
        seedFriend('usr_one', 'online');

        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_one']
        });
        useFriendRosterStore.getState().applyFriendPatch({
            userId: 'usr_one',
            patch: { id: 'usr_one' },
            stateBucket: 'active'
        });
        deferred.resolve({
            id: 'usr_one',
            date_joined: '2026-01-01'
        });
        await waitForStatus('completed');

        expect(
            useFriendRosterStore.getState().friendsById.usr_one
        ).toMatchObject({
            stateBucket: 'active',
            date_joined: '2026-01-01'
        });
    });

    it('does not recreate a friend removed while its profile is loading', async () => {
        const deferred = createDeferred<Record<string, unknown>>();
        mocks.getUserProfile.mockReturnValue(deferred.promise);
        seedFriend('usr_one');
        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_one']
        });

        useFriendRosterStore.getState().removeFriend('usr_one');
        deferred.resolve({
            id: 'usr_one',
            date_joined: '2026-01-01'
        });
        await waitForStatus('completed');

        expect(
            useFriendRosterStore.getState().friendsById.usr_one
        ).toBeUndefined();
        expect(useRuntimeStore.getState().friendProfileLoad).toMatchObject({
            processedFriends: 1,
            loadedFriends: 0,
            failedFriends: 0
        });
    });

    it('minimizes without cancelling and reopens instead of starting twice', async () => {
        const deferred = createDeferred<Record<string, unknown>>();
        mocks.getUserProfile.mockReturnValue(deferred.promise);
        seedFriend('usr_one');
        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_one']
        });

        minimizeFriendProfileLoadDialog();
        expect(useRuntimeStore.getState().friendProfileLoad).toMatchObject({
            status: 'running',
            dialogOpen: false,
            cancelRequested: false
        });
        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_two']
        });
        expect(useRuntimeStore.getState().friendProfileLoad.dialogOpen).toBe(
            true
        );
        expect(mocks.getUserProfile).toHaveBeenCalledTimes(1);

        deferred.resolve({ id: 'usr_one', date_joined: '2026-01-01' });
        await waitForStatus('completed');
    });

    it('cancels in-flight workers without dispatching more friends', async () => {
        const pending: Array<() => void> = [];
        mocks.getUserProfile.mockImplementation(
            ({ userId }: { userId: string }) =>
                new Promise((resolve) => {
                    pending.push(() => resolve({ id: userId }));
                })
        );
        const friendIds = Array.from({ length: 6 }, (_, index) => {
            const userId = `usr_${index}`;
            seedFriend(userId);
            return userId;
        });
        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds
        });
        await vi.waitFor(() => {
            expect(mocks.getUserProfile).toHaveBeenCalledTimes(3);
        });

        cancelFriendProfileLoad();
        expect(useRuntimeStore.getState().friendProfileLoad.status).toBe(
            'cancelling'
        );
        pending.splice(0).forEach((resolve) => resolve());
        await waitForStatus('cancelled');
        expect(mocks.getUserProfile).toHaveBeenCalledTimes(3);
    });

    it('drops a late response after the account endpoint changes', async () => {
        const deferred = createDeferred<Record<string, unknown>>();
        mocks.getUserProfile.mockReturnValue(deferred.promise);
        seedFriend('usr_one');
        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_one']
        });

        useRuntimeStore.getState().setAuthBootstrap({
            currentUserEndpoint: 'https://api.other.test'
        });
        deferred.resolve({
            id: 'usr_one',
            date_joined: '2026-01-01'
        });
        await Promise.resolve();
        await Promise.resolve();

        expect(useRuntimeStore.getState().friendProfileLoad.status).toBe(
            'idle'
        );
        expect(
            useFriendRosterStore.getState().friendsById.usr_one?.date_joined
        ).toBeUndefined();
    });

    it('keeps terminal progress for five seconds before resetting', async () => {
        vi.useFakeTimers();
        seedFriend('usr_one');
        mocks.getUserProfile.mockResolvedValue({
            id: 'usr_one',
            date_joined: '2026-01-01'
        });
        startFriendProfileLoad({
            ownerUserId: OWNER_USER_ID,
            endpoint: ENDPOINT,
            friendIds: ['usr_one']
        });
        await Promise.resolve();
        await Promise.resolve();
        await Promise.resolve();
        await vi.advanceTimersByTimeAsync(0);
        expect(useRuntimeStore.getState().friendProfileLoad.status).toBe(
            'completed'
        );

        await vi.advanceTimersByTimeAsync(4999);
        expect(useRuntimeStore.getState().friendProfileLoad.status).toBe(
            'completed'
        );
        await vi.advanceTimersByTimeAsync(1);
        expect(useRuntimeStore.getState().friendProfileLoad.status).toBe(
            'idle'
        );
    });
});
