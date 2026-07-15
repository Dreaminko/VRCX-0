// @vitest-environment jsdom

import { act, cleanup, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    acceptIncoming: vi.fn(),
    addHistory: vi.fn(),
    applyFriendPatch: vi.fn(),
    bumpRevision: vi.fn(),
    cancelRequest: vi.fn(),
    findIncoming: vi.fn(),
    hideIncoming: vi.fn(),
    notifyMenu: vi.fn(),
    sendRequest: vi.fn(),
    toastError: vi.fn(),
    toastInfo: vi.fn(),
    toastSuccess: vi.fn()
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock('sonner', () => ({
    toast: {
        error: mocks.toastError,
        info: mocks.toastInfo,
        success: mocks.toastSuccess
    }
}));

vi.mock('@/repositories/vrchatFriendRepository', () => ({
    default: {
        cancelFriendRequest: mocks.cancelRequest,
        sendFriendRequest: mocks.sendRequest
    }
}));

vi.mock('@/repositories/friendLogHistoryRepository', () => ({
    default: {
        addFriendLogHistory: mocks.addHistory
    }
}));

vi.mock('@/services/notificationActionService', () => ({
    acceptFriendRequestNotification: mocks.acceptIncoming,
    dismissBoopNotifications: vi.fn(),
    expireNotificationLocally: vi.fn(),
    findIncomingFriendRequestNotification: mocks.findIncoming,
    hideRemoteAndExpireNotification: mocks.hideIncoming
}));

vi.mock('@/services/recentActionService', () => ({
    recordRecentAction: vi.fn()
}));

vi.mock('@/state/friendLogStore', () => ({
    useFriendLogStore: {
        getState: () => ({ bumpRevision: mocks.bumpRevision })
    }
}));

vi.mock('@/state/shellStore', () => ({
    useShellStore: {
        getState: () => ({ notifyMenu: mocks.notifyMenu })
    }
}));

vi.mock('./useUserInviteActions', () => ({
    useUserInviteActions: () => ({
        handleInviteMessageDialogOpenChange: vi.fn(),
        inviteMessageRequest: null,
        selectInviteMessage: vi.fn(),
        sendUserInvite: vi.fn(),
        sendUserInviteRequest: vi.fn()
    })
}));

vi.mock('./useUserModerationActions', () => ({
    useUserModerationActions: () => ({
        setAvatarOverrideModeration: vi.fn(),
        setExtendedUserModeration: vi.fn(),
        setUserModeration: vi.fn()
    })
}));

import { useUserDialogActions } from './useUserDialogActions';

type HookProps = Parameters<typeof useUserDialogActions>[0];
type HookValue = ReturnType<typeof useUserDialogActions>;

function HookHarness({
    onValue,
    props
}: {
    onValue: (value: HookValue) => void;
    props: HookProps;
}) {
    onValue(useUserDialogActions(props));
    return null;
}

function createProps(): HookProps {
    return {
        actionStatusRef: { current: 'idle' },
        activeUserTargetRef: {
            current: {
                userId: 'usr_target',
                endpoint: 'https://api.vrchat.cloud/api/1'
            }
        },
        applyFriendPatch: mocks.applyFriendPatch,
        avatarOverrideState: { hideAvatar: false, showAvatar: false },
        canInviteFromCurrentLocation: false,
        confirm: vi.fn().mockResolvedValue({ ok: true }),
        currentEndpoint: 'https://api.vrchat.cloud/api/1',
        currentInviteLocation: null,
        currentUserId: 'usr_self',
        friendsById: {},
        isCurrentUser: false,
        isFriend: false,
        normalizedCurrentUserId: 'usr_self',
        normalizedUserId: 'usr_target',
        moderationRevisionRef: { current: 0 },
        moderationState: { block: false, mute: false },
        openNonce: 1,
        profile: { id: 'usr_target', displayName: 'Target' },
        setActionStatus: vi.fn(),
        setAvatarOverrideState: vi.fn(),
        setBaseProfile: vi.fn(),
        setExtendedModerationState: vi.fn(),
        setModerationState: vi.fn()
    };
}

describe('useUserDialogActions friend request history', () => {
    let current: HookValue | null;
    let props: HookProps;

    beforeEach(() => {
        vi.clearAllMocks();
        current = null;
        props = createProps();
        mocks.addHistory.mockResolvedValue(undefined);
        mocks.cancelRequest.mockResolvedValue({ json: {} });
        mocks.sendRequest.mockResolvedValue({ json: { success: false } });
        mocks.findIncoming.mockResolvedValue(null);
        render(
            <HookHarness
                onValue={(value) => {
                    current = value;
                }}
                props={props}
            />
        );
    });

    afterEach(() => {
        cleanup();
        vi.restoreAllMocks();
    });

    function actions() {
        if (!current) {
            throw new Error('Hook value is unavailable.');
        }
        return current.actions;
    }

    it('records an outgoing request even when the dialog target changes before the response', async () => {
        mocks.sendRequest.mockImplementation(async () => {
            props.activeUserTargetRef.current = {
                userId: 'usr_other',
                endpoint: props.currentEndpoint
            };
            return { json: { success: false } };
        });

        await act(() => actions().updateFriendRequest('send'));

        expect(mocks.addHistory).toHaveBeenCalledWith('usr_self', {
            created_at: expect.any(String),
            type: 'FriendRequest',
            userId: 'usr_target',
            displayName: 'Target'
        });
    });

    it('records outgoing cancellation but not accepted or declined incoming requests', async () => {
        await act(() => actions().updateFriendRequest('cancel'));
        expect(mocks.addHistory).toHaveBeenLastCalledWith('usr_self', {
            created_at: expect.any(String),
            type: 'CancelFriendRequest',
            userId: 'usr_target',
            displayName: 'Target'
        });

        mocks.addHistory.mockClear();
        mocks.findIncoming.mockResolvedValue({ id: 'not_friend_request' });
        mocks.acceptIncoming.mockResolvedValue({ status: 'accepted' });
        await act(() => actions().updateFriendRequest('accept'));
        await act(() => actions().updateFriendRequest('decline'));
        expect(mocks.addHistory).not.toHaveBeenCalled();
    });

    it('does not write history when the remote request fails', async () => {
        mocks.sendRequest.mockRejectedValue(new Error('remote failed'));

        await act(() => actions().updateFriendRequest('send'));

        expect(mocks.addHistory).not.toHaveBeenCalled();
        expect(mocks.toastError).toHaveBeenCalled();
    });

    it('keeps a successful remote action successful when history persistence fails', async () => {
        vi.spyOn(console, 'warn').mockImplementation(() => {});
        mocks.addHistory.mockRejectedValue(new Error('database failed'));

        await act(() => actions().updateFriendRequest('send'));

        expect(mocks.toastSuccess).toHaveBeenCalledWith(
            'dialog.user.toast.friend_request_sent'
        );
        expect(mocks.toastError).not.toHaveBeenCalled();
    });
});
