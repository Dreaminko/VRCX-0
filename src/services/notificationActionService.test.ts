import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    queryNotifications: vi.fn(),
    expireNotification: vi.fn(),
    hideRemoteNotification: vi.fn(),
    acceptFriendRequest: vi.fn(),
    sendNotificationResponse: vi.fn(),
    sendInviteResponse: vi.fn(),
    sendInviteResponsePhoto: vi.fn(),
    registerFriendLogExplicitAddIntent: vi.fn(),
    recordFriendLogFriendByUserId: vi.fn(),
    sendBoopToUser: vi.fn(),
    sendInviteToLocation: vi.fn(),
    notifyMenu: vi.fn(),
    clearFriendLogAddIntent: vi.fn()
}));

vi.mock('@/repositories/notificationPersistenceRepository', () => ({
    default: {
        queryNotifications: mocks.queryNotifications,
        expireNotification: mocks.expireNotification,
        hideRemoteNotification: mocks.hideRemoteNotification,
        acceptFriendRequest: mocks.acceptFriendRequest,
        sendNotificationResponse: mocks.sendNotificationResponse,
        sendInviteResponse: mocks.sendInviteResponse,
        sendInviteResponsePhoto: mocks.sendInviteResponsePhoto
    }
}));

vi.mock('@/state/shellStore', () => ({
    useShellStore: {
        getState: () => ({ notifyMenu: mocks.notifyMenu })
    }
}));

vi.mock('./friendBootstrapService', () => ({
    registerFriendLogExplicitAddIntent:
        mocks.registerFriendLogExplicitAddIntent,
    recordFriendLogFriendByUserId: mocks.recordFriendLogFriendByUserId
}));

vi.mock('./inviteDeliveryService', () => ({
    sendBoopToUser: mocks.sendBoopToUser,
    sendInviteToLocation: mocks.sendInviteToLocation
}));

const endpoint = 'https://api.example.test/api/1';
const notification = {
    id: 'notif_target',
    version: 2,
    type: 'boop',
    senderUserId: 'usr_sender',
    senderUsername: 'Sender'
};

function deferred<T>() {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>((nextResolve) => {
        resolve = nextResolve;
    });
    return { promise, resolve };
}

describe('notificationActionService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.queryNotifications.mockResolvedValue([]);
        mocks.expireNotification.mockResolvedValue(undefined);
        mocks.hideRemoteNotification.mockResolvedValue(undefined);
        mocks.acceptFriendRequest.mockResolvedValue(undefined);
        mocks.sendNotificationResponse.mockResolvedValue(undefined);
        mocks.sendInviteResponse.mockResolvedValue(undefined);
        mocks.sendInviteResponsePhoto.mockResolvedValue(undefined);
        mocks.registerFriendLogExplicitAddIntent.mockReturnValue(
            mocks.clearFriendLogAddIntent
        );
        mocks.recordFriendLogFriendByUserId.mockResolvedValue({
            historyCount: 0
        });
        mocks.sendBoopToUser.mockResolvedValue(undefined);
        mocks.sendInviteToLocation.mockResolvedValue(undefined);
    });

    it('dismisses matching boops before sending a reply, then cleans up the target', async () => {
        const previousHide = deferred<void>();
        const previousExpire = deferred<void>();
        mocks.hideRemoteNotification
            .mockReturnValueOnce(previousHide.promise)
            .mockResolvedValue(undefined);
        mocks.expireNotification
            .mockReturnValueOnce(previousExpire.promise)
            .mockResolvedValue(undefined);
        const previousBoop = {
            id: 'notif_previous',
            version: 1,
            type: 'boop',
            senderUserId: 'usr_sender',
            link: 'user:usr_sender',
            expired: false
        };
        mocks.queryNotifications.mockResolvedValue([previousBoop]);
        const { sendBoopReplyNotification } =
            await import('./notificationActionService');

        const reply = sendBoopReplyNotification({
            currentUserId: 'usr_self',
            endpoint,
            notification,
            emojiId: 'emoji_wave'
        });

        await vi.waitFor(() =>
            expect(mocks.hideRemoteNotification).toHaveBeenCalledTimes(1)
        );
        expect(mocks.expireNotification).not.toHaveBeenCalled();
        expect(mocks.sendBoopToUser).not.toHaveBeenCalled();

        previousHide.resolve();
        await vi.waitFor(() =>
            expect(mocks.expireNotification).toHaveBeenCalledTimes(1)
        );
        expect(mocks.sendBoopToUser).not.toHaveBeenCalled();

        previousExpire.resolve();
        await reply;

        expect(mocks.hideRemoteNotification).toHaveBeenNthCalledWith(1, {
            id: 'notif_previous',
            version: 1,
            type: 'boop',
            senderUserId: 'usr_sender',
            endpoint
        });
        expect(mocks.expireNotification).toHaveBeenNthCalledWith(1, {
            userId: 'usr_self',
            id: 'notif_previous'
        });
        expect(mocks.sendBoopToUser).toHaveBeenCalledWith({
            userId: 'usr_sender',
            emojiId: 'emoji_wave',
            endpoint
        });
        expect(mocks.hideRemoteNotification).toHaveBeenNthCalledWith(2, {
            id: 'notif_target',
            version: 2,
            type: 'boop',
            senderUserId: 'usr_sender',
            endpoint
        });
        expect(mocks.expireNotification).toHaveBeenNthCalledWith(2, {
            userId: 'usr_self',
            id: 'notif_target'
        });
    });

    it('finishes dismiss cleanup before surfacing a boop send failure', async () => {
        const previousHide = deferred<void>();
        const previousExpire = deferred<void>();
        mocks.hideRemoteNotification.mockReturnValue(previousHide.promise);
        mocks.expireNotification.mockReturnValue(previousExpire.promise);
        mocks.queryNotifications.mockResolvedValue([
            {
                id: 'notif_previous',
                version: 1,
                type: 'boop',
                senderUserId: 'usr_sender',
                link: 'user:usr_sender',
                expired: false
            }
        ]);
        mocks.sendBoopToUser.mockRejectedValue(new Error('send failed'));
        const { sendBoopReplyNotification } =
            await import('./notificationActionService');

        const reply = sendBoopReplyNotification({
            currentUserId: 'usr_self',
            endpoint,
            notification
        });

        await vi.waitFor(() =>
            expect(mocks.hideRemoteNotification).toHaveBeenCalledTimes(1)
        );
        expect(mocks.expireNotification).not.toHaveBeenCalled();
        expect(mocks.sendBoopToUser).not.toHaveBeenCalled();

        previousHide.resolve();
        await vi.waitFor(() =>
            expect(mocks.expireNotification).toHaveBeenCalledTimes(1)
        );
        expect(mocks.sendBoopToUser).not.toHaveBeenCalled();

        previousExpire.resolve();
        await expect(reply).rejects.toThrow('send failed');

        expect(mocks.hideRemoteNotification).toHaveBeenCalledTimes(1);
        expect(mocks.expireNotification).toHaveBeenCalledTimes(1);
        expect(mocks.expireNotification).toHaveBeenCalledWith({
            userId: 'usr_self',
            id: 'notif_previous'
        });
    });

    it.each([
        { version: 1, expires: false },
        { version: 2, expires: true }
    ])(
        'uses v$version response failure expiration semantics',
        async ({ version, expires }) => {
            const responseError = new Error('response failed');
            mocks.sendNotificationResponse.mockRejectedValue(responseError);
            const { sendNotificationButtonResponse } =
                await import('./notificationActionService');

            await expect(
                sendNotificationButtonResponse({
                    currentUserId: 'usr_self',
                    endpoint,
                    notification: { ...notification, version },
                    response: { type: 'accept', data: 'payload' }
                })
            ).rejects.toBe(responseError);

            expect(mocks.sendNotificationResponse).toHaveBeenCalledWith({
                id: 'notif_target',
                responseType: 'accept',
                responseData: 'payload',
                endpoint
            });
            expect(mocks.expireNotification).toHaveBeenCalledTimes(
                expires ? 1 : 0
            );
            if (expires) {
                expect(mocks.expireNotification).toHaveBeenCalledWith({
                    userId: 'usr_self',
                    id: 'notif_target'
                });
            }
        }
    );

    it('accepts a friend request, records friend history, notifies the menu, and expires it', async () => {
        const accepted = deferred<void>();
        const recorded = deferred<{ historyCount: number }>();
        mocks.acceptFriendRequest.mockReturnValue(accepted.promise);
        mocks.recordFriendLogFriendByUserId.mockResolvedValue({
            historyCount: 1
        });
        mocks.recordFriendLogFriendByUserId.mockReturnValue(recorded.promise);
        const { acceptFriendRequestNotification } =
            await import('./notificationActionService');

        const action = acceptFriendRequestNotification({
            currentUserId: 'usr_self',
            endpoint,
            notification,
            stateBucket: 'online'
        });

        expect(mocks.recordFriendLogFriendByUserId).not.toHaveBeenCalled();
        expect(mocks.expireNotification).not.toHaveBeenCalled();
        accepted.resolve();
        await vi.waitFor(() =>
            expect(mocks.recordFriendLogFriendByUserId).toHaveBeenCalledTimes(1)
        );
        expect(mocks.expireNotification).not.toHaveBeenCalled();
        recorded.resolve({ historyCount: 1 });
        await expect(action).resolves.toEqual({ status: 'accepted' });

        expect(mocks.registerFriendLogExplicitAddIntent).toHaveBeenCalledWith({
            currentUserId: 'usr_self',
            targetUserId: 'usr_sender'
        });
        expect(mocks.acceptFriendRequest).toHaveBeenCalledWith({
            id: 'notif_target',
            endpoint
        });
        expect(mocks.recordFriendLogFriendByUserId).toHaveBeenCalledWith({
            currentUserId: 'usr_self',
            targetUserId: 'usr_sender',
            targetUser: {
                id: 'usr_sender',
                displayName: 'Sender'
            },
            stateBucket: 'online'
        });
        expect(mocks.notifyMenu).toHaveBeenCalledWith('friend-log');
        expect(mocks.expireNotification).toHaveBeenCalledWith({
            userId: 'usr_self',
            id: 'notif_target'
        });
        expect(mocks.clearFriendLogAddIntent).not.toHaveBeenCalled();
    });

    it('treats a missing remote friend request as resolved locally', async () => {
        mocks.acceptFriendRequest.mockRejectedValue(
            Object.assign(new Error('not found'), { status: 404 })
        );
        const { acceptFriendRequestNotification } =
            await import('./notificationActionService');

        await expect(
            acceptFriendRequestNotification({
                currentUserId: 'usr_self',
                endpoint,
                notification
            })
        ).resolves.toEqual({ status: 'not-found' });

        expect(mocks.clearFriendLogAddIntent).toHaveBeenCalledTimes(1);
        expect(mocks.recordFriendLogFriendByUserId).not.toHaveBeenCalled();
        expect(mocks.expireNotification).toHaveBeenCalledWith({
            userId: 'usr_self',
            id: 'notif_target'
        });
    });

    it('keeps a successful accept when friend log recording fails', async () => {
        const logError = new Error('log failed');
        mocks.recordFriendLogFriendByUserId.mockRejectedValue(logError);
        const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
        const { acceptFriendRequestNotification } =
            await import('./notificationActionService');

        await expect(
            acceptFriendRequestNotification({
                currentUserId: 'usr_self',
                endpoint,
                notification
            })
        ).resolves.toEqual({ status: 'accepted' });

        expect(mocks.clearFriendLogAddIntent).toHaveBeenCalledTimes(1);
        expect(warn).toHaveBeenCalledWith(
            'Friend log add recording failed:',
            logError
        );
        expect(mocks.expireNotification).toHaveBeenCalledWith({
            userId: 'usr_self',
            id: 'notif_target'
        });
        warn.mockRestore();
    });

    it('rejects invalid action input before crossing repository or delivery boundaries', async () => {
        const {
            expireNotificationLocally,
            findIncomingFriendRequestNotification,
            sendBoopReplyNotification,
            sendInviteResponseNotification
        } = await import('./notificationActionService');

        await expect(
            expireNotificationLocally({
                currentUserId: 'usr_self',
                notification: null
            })
        ).rejects.toThrow('Notification action requires a notification.');
        await expect(
            sendBoopReplyNotification({
                currentUserId: 'usr_self',
                notification: { id: 'notif_without_sender' }
            })
        ).rejects.toThrow('Cannot send boop: no sender user id is available.');
        await expect(
            sendInviteResponseNotification({
                currentUserId: 'usr_self',
                notification,
                responseSlot: 'invalid'
            })
        ).rejects.toThrow('Response slot must be a number.');
        await expect(
            findIncomingFriendRequestNotification({
                currentUserId: ' ',
                targetUserId: 'usr_sender'
            })
        ).resolves.toBeNull();

        expect(mocks.queryNotifications).not.toHaveBeenCalled();
        expect(mocks.expireNotification).not.toHaveBeenCalled();
        expect(mocks.sendBoopToUser).not.toHaveBeenCalled();
        expect(mocks.sendInviteResponse).not.toHaveBeenCalled();
        expect(mocks.sendInviteResponsePhoto).not.toHaveBeenCalled();
    });
});
