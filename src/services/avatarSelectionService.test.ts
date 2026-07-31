import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    selectAvatar: vi.fn(),
    selectFallbackAvatar: vi.fn()
}));

vi.mock('@/repositories/avatarProfileRepository', () => ({
    default: {
        selectAvatar: mocks.selectAvatar,
        selectFallbackAvatar: mocks.selectFallbackAvatar
    }
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
        useRuntimeStore.getState().resetRuntimeState();
    });

    it('invokes the avatar selection command and reports the backend apply result', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        });
        mocks.selectAvatar.mockResolvedValue({
            applied: true,
            status: 200,
            json: { id: 'usr_self', currentAvatar: 'avtr_new' }
        });

        await expect(selectAvatar('avtr_new')).resolves.toMatchObject({
            applied: true
        });

        expect(mocks.selectAvatar).toHaveBeenCalledWith({
            avatarId: 'avtr_new'
        });
    });

    it('reports a dropped apply when the backend sequence gate rejects the response', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        });
        mocks.selectAvatar.mockResolvedValue({
            applied: false,
            status: 200,
            json: { id: 'usr_self', currentAvatar: 'avtr_new' }
        });

        await expect(selectAvatar('avtr_new')).resolves.toMatchObject({
            applied: false
        });
    });

    it('propagates selection failures', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        });
        mocks.selectAvatar.mockRejectedValue(new Error('selection failed'));

        await expect(selectAvatar('avtr_new')).rejects.toThrow(
            'selection failed'
        );
    });

    it('rejects an invalid current-user response', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_old'
        });
        mocks.selectAvatar.mockResolvedValue({
            applied: false,
            status: 200,
            json: { currentAvatar: 'avtr_new' }
        });

        await expect(selectAvatar('avtr_new')).rejects.toThrow(
            'invalid current user'
        );
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
        'reports an unapplied selection after the authenticated $changedTarget changes',
        async ({ currentUserId, currentUserEndpoint }) => {
            setAuthenticatedSnapshot({
                id: 'usr_self',
                currentAvatar: 'avtr_old'
            });
            const response = deferred<{
                applied: boolean;
                json: Record<string, unknown>;
                status: number;
            }>();
            mocks.selectAvatar.mockReturnValue(response.promise);
            const selection = selectAvatar('avtr_new');

            useRuntimeStore.getState().setAuthBootstrap({
                currentUserId,
                currentUserEndpoint,
                currentUserSnapshot: {
                    id: currentUserId,
                    currentAvatar: 'avtr_other'
                }
            });
            response.resolve({
                applied: true,
                status: 200,
                json: {
                    id: 'usr_self',
                    currentAvatar: 'avtr_new'
                }
            });
            await expect(selection).resolves.toMatchObject({ applied: false });
        }
    );

    it('rejects selection without an authenticated current user', async () => {
        await expect(selectAvatar('avtr_new')).rejects.toThrow(
            'requires a current user'
        );

        expect(mocks.selectAvatar).not.toHaveBeenCalled();
    });

    it('invokes the fallback selection command', async () => {
        setAuthenticatedSnapshot({
            id: 'usr_self',
            currentAvatar: 'avtr_current',
            fallbackAvatar: 'avtr_old_fallback'
        });
        mocks.selectFallbackAvatar.mockResolvedValue({
            applied: true,
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
    });
});
