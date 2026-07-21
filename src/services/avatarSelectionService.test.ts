import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    selectAvatar: vi.fn(),
    selectFallbackAvatar: vi.fn(),
    recordCurrentUserSnapshot: vi.fn()
}));

vi.mock('@/repositories/avatarProfileRepository', () => ({
    default: {
        selectAvatar: mocks.selectAvatar,
        selectFallbackAvatar: mocks.selectFallbackAvatar
    }
}));

vi.mock('./domainIngestionService', () => ({
    recordCurrentUserSnapshot: mocks.recordCurrentUserSnapshot
}));

import { useRuntimeStore } from '@/state/runtimeStore';

import { selectAvatar, selectFallbackAvatar } from './avatarSelectionService';

const ENDPOINT = 'https://api.vrchat.cloud/api/1';

function deferred<T>() {
    let resolve: (value: T) => void = () => {
        throw new Error('Deferred promise was not initialized.');
    };
    const promise = new Promise<T>((next) => {
        resolve = next;
    });
    return { promise, resolve };
}

function setAuthenticatedSnapshot(snapshot: Record<string, unknown>) {
    useRuntimeStore.getState().setAuthBootstrap({
        currentUserId: 'usr_self',
        currentUserDisplayName: 'Self',
        currentUserEndpoint: ENDPOINT,
        currentUserSnapshot: snapshot
    });
}

describe('avatarSelectionService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.useRealTimers();
        useRuntimeStore.getState().resetRuntimeState();
    });

    it('keeps the current avatar unchanged until the selection response succeeds', async () => {
        vi.useFakeTimers();
        vi.setSystemTime(5_000);
        const previousSnapshot = {
            id: 'usr_self',
            displayName: 'Self',
            currentAvatar: 'avtr_old',
            location: 'wrld_live:instance(usr_self)',
            stateBucket: 'online',
            statusDescription: 'Before request',
            $isVRCPlus: false,
            $previousAvatarSwapTime: 1_000
        };
        setAuthenticatedSnapshot(previousSnapshot);
        useRuntimeStore.getState().setGameState({ isGameRunning: true });
        const response = deferred<{
            json: Record<string, unknown>;
            status: number;
        }>();
        mocks.selectAvatar.mockReturnValue(response.promise);

        const selection = selectAvatar('avtr_new');

        expect(useRuntimeStore.getState().auth.currentUserSnapshot).toBe(
            previousSnapshot
        );

        useRuntimeStore.getState().setAuthBootstrap({
            currentUserSnapshot: {
                ...previousSnapshot,
                statusDescription: 'Changed while pending',
                $isVRCPlus: true
            }
        });
        response.resolve({
            status: 200,
            json: {
                id: 'usr_self',
                displayName: 'Updated Self',
                currentAvatar: 'avtr_new',
                stateBucket: 'offline',
                statusDescription: 'Stale response',
                $isVRCPlus: false
            }
        });
        await expect(selection).resolves.toMatchObject({
            applied: true
        });

        const nextSnapshot =
            useRuntimeStore.getState().auth.currentUserSnapshot;
        expect(nextSnapshot).toMatchObject({
            id: 'usr_self',
            displayName: 'Updated Self',
            currentAvatar: 'avtr_new',
            location: previousSnapshot.location,
            stateBucket: 'online',
            statusDescription: 'Changed while pending',
            $isVRCPlus: true,
            $previousAvatarSwapTime: 5_000
        });
        expect(useRuntimeStore.getState().auth.currentUserDisplayName).toBe(
            'Updated Self'
        );
        expect(mocks.recordCurrentUserSnapshot).toHaveBeenCalledWith(
            nextSnapshot,
            { endpoint: ENDPOINT }
        );
    });

    it('leaves the current avatar unchanged when selection fails', async () => {
        const previousSnapshot = {
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        };
        setAuthenticatedSnapshot(previousSnapshot);
        mocks.selectAvatar.mockRejectedValue(new Error('selection failed'));

        await expect(selectAvatar('avtr_new')).rejects.toThrow(
            'selection failed'
        );

        expect(useRuntimeStore.getState().auth.currentUserSnapshot).toBe(
            previousSnapshot
        );
        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
    });

    it('rejects an invalid current-user response without changing runtime auth', async () => {
        const previousSnapshot = {
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        };
        setAuthenticatedSnapshot(previousSnapshot);
        mocks.selectAvatar.mockResolvedValue({
            status: 200,
            json: { currentAvatar: 'avtr_new' }
        });

        await expect(selectAvatar('avtr_new')).rejects.toThrow(
            'invalid current user'
        );

        expect(useRuntimeStore.getState().auth.currentUserSnapshot).toBe(
            previousSnapshot
        );
        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
    });

    it.each([
        {
            changedTarget: 'user',
            currentUserId: 'usr_other',
            currentUserEndpoint: ENDPOINT
        },
        {
            changedTarget: 'endpoint',
            currentUserId: 'usr_self',
            currentUserEndpoint: 'https://example.test/api/1'
        }
    ])(
        'does not apply a response after the authenticated $changedTarget changes',
        async ({ currentUserId, currentUserEndpoint }) => {
            setAuthenticatedSnapshot({
                id: 'usr_self',
                currentAvatar: 'avtr_old'
            });
            const response = deferred<{
                json: Record<string, unknown>;
                status: number;
            }>();
            mocks.selectAvatar.mockReturnValue(response.promise);
            const selection = selectAvatar('avtr_new');
            const otherSnapshot = {
                id: currentUserId,
                currentAvatar: 'avtr_other'
            };

            useRuntimeStore.getState().setAuthBootstrap({
                currentUserId,
                currentUserEndpoint,
                currentUserSnapshot: otherSnapshot
            });
            response.resolve({
                status: 200,
                json: {
                    id: 'usr_self',
                    currentAvatar: 'avtr_new'
                }
            });
            await expect(selection).resolves.toMatchObject({ applied: false });

            expect(useRuntimeStore.getState().auth.currentUserSnapshot).toBe(
                otherSnapshot
            );
            expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
        }
    );

    it('does not let an older selection response replace the latest result', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        });
        const firstResponse = deferred<{
            json: Record<string, unknown>;
            status: number;
        }>();
        const latestResponse = deferred<{
            json: Record<string, unknown>;
            status: number;
        }>();
        mocks.selectAvatar
            .mockReturnValueOnce(firstResponse.promise)
            .mockReturnValueOnce(latestResponse.promise);
        const firstSelection = selectAvatar('avtr_first');
        const latestSelection = selectAvatar('avtr_latest');

        latestResponse.resolve({
            status: 200,
            json: {
                id: 'usr_self',
                currentAvatar: 'avtr_latest'
            }
        });
        await expect(latestSelection).resolves.toMatchObject({
            applied: true
        });
        firstResponse.resolve({
            status: 200,
            json: {
                id: 'usr_self',
                currentAvatar: 'avtr_first'
            }
        });
        await expect(firstSelection).resolves.toMatchObject({
            applied: false
        });

        expect(
            useRuntimeStore.getState().auth.currentUserSnapshot
        ).toMatchObject({ currentAvatar: 'avtr_latest' });
        expect(mocks.recordCurrentUserSnapshot).toHaveBeenCalledTimes(1);
    });

    it('applies a newer successful avatar after an earlier success', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        });
        const firstResponse = deferred<{
            json: Record<string, unknown>;
            status: number;
        }>();
        const latestResponse = deferred<{
            json: Record<string, unknown>;
            status: number;
        }>();
        mocks.selectAvatar
            .mockReturnValueOnce(firstResponse.promise)
            .mockReturnValueOnce(latestResponse.promise);
        const firstSelection = selectAvatar('avtr_first');
        const latestSelection = selectAvatar('avtr_latest');

        firstResponse.resolve({
            status: 200,
            json: {
                id: 'usr_self',
                currentAvatar: 'avtr_first'
            }
        });
        await expect(firstSelection).resolves.toMatchObject({ applied: true });
        latestResponse.resolve({
            status: 200,
            json: {
                id: 'usr_self',
                currentAvatar: 'avtr_latest'
            }
        });
        await expect(latestSelection).resolves.toMatchObject({ applied: true });

        expect(
            useRuntimeStore.getState().auth.currentUserSnapshot
        ).toMatchObject({ currentAvatar: 'avtr_latest' });
    });

    it('applies an earlier success when the newer selection fails', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        });
        const firstResponse = deferred<{
            json: Record<string, unknown>;
            status: number;
        }>();
        mocks.selectAvatar
            .mockReturnValueOnce(firstResponse.promise)
            .mockRejectedValueOnce(new Error('latest selection failed'));
        const firstSelection = selectAvatar('avtr_first');
        const latestSelection = selectAvatar('avtr_latest');

        await expect(latestSelection).rejects.toThrow(
            'latest selection failed'
        );
        firstResponse.resolve({
            status: 200,
            json: {
                id: 'usr_self',
                currentAvatar: 'avtr_first'
            }
        });
        await expect(firstSelection).resolves.toMatchObject({ applied: true });

        expect(
            useRuntimeStore.getState().auth.currentUserSnapshot
        ).toMatchObject({ currentAvatar: 'avtr_first' });
    });

    it('does not apply a response for a different current user', async () => {
        const previousSnapshot = {
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        };
        setAuthenticatedSnapshot(previousSnapshot);
        mocks.selectAvatar.mockResolvedValue({
            status: 200,
            json: {
                id: 'usr_other',
                currentAvatar: 'avtr_other'
            }
        });

        await expect(selectAvatar('avtr_other')).resolves.toMatchObject({
            applied: false
        });

        expect(useRuntimeStore.getState().auth.currentUserSnapshot).toBe(
            previousSnapshot
        );
        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
    });

    it('rejects selection without an authenticated current user', async () => {
        await expect(selectAvatar('avtr_new')).rejects.toThrow(
            'requires a current user'
        );

        expect(mocks.selectAvatar).not.toHaveBeenCalled();
        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
    });

    it('applies the CurrentUser returned by fallback selection', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_current',
            fallbackAvatar: 'avtr_old_fallback'
        });
        mocks.selectFallbackAvatar.mockResolvedValue({
            status: 200,
            json: {
                id: 'usr_self',
                currentAvatar: 'avtr_current',
                fallbackAvatar: 'avtr_new_fallback'
            }
        });

        await expect(
            selectFallbackAvatar('avtr_new_fallback')
        ).resolves.toMatchObject({ applied: true });

        expect(mocks.selectFallbackAvatar).toHaveBeenCalledWith({
            avatarId: 'avtr_new_fallback'
        });
        expect(
            useRuntimeStore.getState().auth.currentUserSnapshot
        ).toMatchObject({ fallbackAvatar: 'avtr_new_fallback' });
    });

    it('does not supersede current-avatar and fallback-avatar selections across kinds', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_old',
            fallbackAvatar: 'avtr_old_fallback'
        });
        const avatarResponse = deferred<{
            json: Record<string, unknown>;
            status: number;
        }>();
        mocks.selectAvatar.mockReturnValue(avatarResponse.promise);
        mocks.selectFallbackAvatar.mockResolvedValue({
            status: 200,
            json: {
                id: 'usr_self',
                currentAvatar: 'avtr_old',
                fallbackAvatar: 'avtr_new_fallback'
            }
        });
        const avatarSelection = selectAvatar('avtr_new');

        await expect(
            selectFallbackAvatar('avtr_new_fallback')
        ).resolves.toMatchObject({ applied: true });
        avatarResponse.resolve({
            status: 200,
            json: {
                id: 'usr_self',
                currentAvatar: 'avtr_new',
                fallbackAvatar: 'avtr_old_fallback'
            }
        });
        await expect(avatarSelection).resolves.toMatchObject({ applied: true });

        expect(
            useRuntimeStore.getState().auth.currentUserSnapshot
        ).toMatchObject({
            currentAvatar: 'avtr_new',
            fallbackAvatar: 'avtr_new_fallback'
        });
    });
});
