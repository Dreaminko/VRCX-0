// @vitest-environment jsdom

import type { Dispatch, SetStateAction } from 'react';
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { FriendListRow } from './friendListRows';

const mocks = vi.hoisted(() => ({
    applyFriendPatch: vi.fn(),
    confirm: vi.fn(),
    deleteFriend: vi.fn(),
    deleteFriends: vi.fn(),
    runtimeState: {
        auth: {
            currentUserId: 'usr_self',
            currentUserEndpoint: 'https://api.vrchat.cloud/api/1',
            currentUserSnapshot: { id: 'usr_self' }
        },
        mutualGraph: {
            runId: 0,
            status: 'idle',
            ownerUserId: '',
            processedFriends: 0,
            totalFriends: 0
        },
        friendProfileLoad: { status: 'idle' }
    },
    friendState: {
        applyFriendPatch: vi.fn(),
        friendsById: {} as Record<string, FriendListRow>
    },
    toastError: vi.fn(),
    toastSuccess: vi.fn(),
    toastWarning: vi.fn()
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock('sonner', () => ({
    toast: {
        error: mocks.toastError,
        info: vi.fn(),
        success: mocks.toastSuccess,
        warning: mocks.toastWarning
    }
}));

vi.mock('@/state/runtimeStore', () => {
    const useRuntimeStore = (
        selector: (state: typeof mocks.runtimeState) => unknown
    ) => selector(mocks.runtimeState);
    useRuntimeStore.getState = () => mocks.runtimeState;
    return { useRuntimeStore };
});

vi.mock('@/state/friendRosterStore', () => {
    const useFriendRosterStore = (
        selector: (state: typeof mocks.friendState) => unknown
    ) => selector(mocks.friendState);
    useFriendRosterStore.getState = () => mocks.friendState;
    return { useFriendRosterStore };
});

vi.mock('@/state/modalStore', () => ({
    useModalStore: (
        selector: (state: { confirm: typeof mocks.confirm }) => unknown
    ) => selector({ confirm: mocks.confirm })
}));

vi.mock('@/services/friendRelationshipService', () => ({
    default: {
        deleteFriend: mocks.deleteFriend,
        deleteFriends: mocks.deleteFriends
    }
}));

vi.mock('@/repositories/mutualGraphPersistenceRepository', () => ({
    default: { getSnapshot: vi.fn() }
}));

vi.mock('@/services/dialogService', () => ({ openUserDialog: vi.fn() }));
vi.mock('@/services/friendProfileLoadService', () => ({
    openFriendProfileLoadDialog: vi.fn(),
    startFriendProfileLoad: vi.fn()
}));
vi.mock('@/services/mutualGraphFetchService', () => ({
    startMutualGraphFetch: vi.fn()
}));

import { useFriendListRowActions } from './useFriendListRowActions';

const friend: FriendListRow = {
    id: 'usr_friend',
    displayName: 'Friend',
    stateBucket: 'online'
};

function renderActions() {
    let deletingFriendIds = new Set<string>();
    let selectedFriendIds = new Set(['usr_friend']);
    const setDeletingFriendIds = vi.fn<
        Dispatch<SetStateAction<Set<string>>>
    >((next) => {
        deletingFriendIds =
            typeof next === 'function' ? next(deletingFriendIds) : next;
    });
    const setSelectedFriendIds = vi.fn<
        Dispatch<SetStateAction<Set<string>>>
    >((next) => {
        selectedFriendIds =
            typeof next === 'function' ? next(selectedFriendIds) : next;
    });
    const hook = renderHook(() =>
        useFriendListRowActions({
            filteredRows: [friend],
            resetTableLayout: vi.fn(),
            rosterRows: [friend],
            selectedFriendIds,
            setDeletingFriendIds,
            setIsBulkDeleting: vi.fn(),
            setMutualProgress: vi.fn(),
            setSelectedFriendIds
        })
    );
    return {
        ...hook,
        deletingFriendIds: () => deletingFriendIds,
        selectedFriendIds: () => selectedFriendIds,
        setDeletingFriendIds
    };
}

describe('useFriendListRowActions', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.friendState.friendsById = { usr_friend: friend };
    });

    it('does not unfriend when destructive confirmation is cancelled', async () => {
        mocks.confirm.mockResolvedValue({ ok: false, reason: 'cancelled' });
        const { result } = renderActions();

        await act(async () => result.current.confirmDeleteFriend(friend));

        expect(mocks.deleteFriend).not.toHaveBeenCalled();
        expect(mocks.toastSuccess).not.toHaveBeenCalled();
    });

    it('locks the row, removes the selection, and warns on partial success', async () => {
        mocks.confirm.mockResolvedValue({ ok: true, value: undefined });
        mocks.deleteFriend.mockResolvedValue({
            stale: false,
            localError: new Error('local persistence failed')
        });
        const rendered = renderActions();

        await act(async () =>
            rendered.result.current.confirmDeleteFriend(friend)
        );

        expect(mocks.deleteFriend).toHaveBeenCalledWith({
            friend,
            userId: 'usr_friend',
            endpoint: 'https://api.vrchat.cloud/api/1',
            currentUserId: 'usr_self'
        });
        expect(rendered.selectedFriendIds()).not.toContain('usr_friend');
        expect(rendered.deletingFriendIds()).not.toContain('usr_friend');
        expect(rendered.setDeletingFriendIds).toHaveBeenCalledTimes(2);
        expect(mocks.toastWarning).toHaveBeenCalledWith(
            'dialog.user.toast.applied_on_vrchat_but_local_update_failed'
        );
        expect(mocks.toastSuccess).not.toHaveBeenCalled();
    });
});
