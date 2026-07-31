import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { NotificationActionOutcome } from '@/platform/tauri/bindings';

const mocks = vi.hoisted(() => ({
    queryNotifications: vi.fn(),
    expireNotification: vi.fn(),
    appSocialFriendRequestAccept: vi.fn(),
    appNotificationHideAndExpire: vi.fn(),
    appNotificationRequestInviteAccept: vi.fn(),
    appNotificationInviteResponseSend: vi.fn(),
    appNotificationBoopDismiss: vi.fn(),
    appNotificationBoopReply: vi.fn(),
    appNotificationRespondAndExpire: vi.fn(),
    getWorlds: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appSocialFriendRequestAccept: mocks.appSocialFriendRequestAccept,
        appNotificationHideAndExpire: mocks.appNotificationHideAndExpire,
        appNotificationRequestInviteAccept:
            mocks.appNotificationRequestInviteAccept,
        appNotificationInviteResponseSend:
            mocks.appNotificationInviteResponseSend,
        appNotificationBoopDismiss: mocks.appNotificationBoopDismiss,
        appNotificationBoopReply: mocks.appNotificationBoopReply,
        appNotificationRespondAndExpire: mocks.appNotificationRespondAndExpire
    }
}));

vi.mock('@/repositories/notificationPersistenceRepository', () => ({
    default: {
        queryNotifications: mocks.queryNotifications,
        expireNotification: mocks.expireNotification
    }
}));

vi.mock('@/repositories/vrchatSearchRepository', () => ({
    default: {
        getWorlds: mocks.getWorlds
    }
}));

const endpoint = 'https://api.example.test/api/1';
const notification = {
    id: 'notif_target',
    version: 2,
    type: 'boop',
    senderUserId: 'usr_sender',
    senderUsername: 'Sender'
};

function outcome(
    overrides: Partial<NotificationActionOutcome> = {}
): NotificationActionOutcome {
    return {
        status: 'applied',
        expiredIds: [],
        sentPhoto: false,
        remoteError: null,
        localError: null,
        ...overrides
    };
}

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
        mocks.appSocialFriendRequestAccept.mockResolvedValue({
            status: 'applied',
            targetUserId: 'usr_sender'
        });
        mocks.appNotificationHideAndExpire.mockResolvedValue(outcome());
        mocks.appNotificationRequestInviteAccept.mockResolvedValue(outcome());
        mocks.appNotificationInviteResponseSend.mockResolvedValue(outcome());
        mocks.appNotificationBoopDismiss.mockResolvedValue(outcome());
        mocks.appNotificationBoopReply.mockResolvedValue(outcome());
        mocks.appNotificationRespondAndExpire.mockResolvedValue(outcome());
        mocks.getWorlds.mockResolvedValue({ json: { name: 'World Name' } });
    });

    it('sends a boop reply through the single backend chain command', async () => {
        mocks.appNotificationBoopReply.mockResolvedValue(
            outcome({ expiredIds: ['notif_previous', 'notif_target'] })
        );
        const { sendBoopReplyNotification } =
            await import('./notificationActionService');

        await sendBoopReplyNotification({
            currentUserId: 'usr_self',
            notification,
            emojiId: 'emoji_wave'
        });

        expect(mocks.appNotificationBoopReply).toHaveBeenCalledWith({
            ownerUserId: 'usr_self',
            target: {
                id: 'notif_target',
                version: 2,
                type: 'boop',
                senderUserId: 'usr_sender'
            },
            emojiId: 'emoji_wave'
        });
    });

    it('surfaces a boop send failure reported by the backend chain', async () => {
        mocks.appNotificationBoopReply.mockResolvedValue(
            outcome({
                status: 'remoteFailed',
                expiredIds: ['notif_previous'],
                remoteError: 'send failed'
            })
        );
        const { sendBoopReplyNotification } =
            await import('./notificationActionService');

        await expect(
            sendBoopReplyNotification({
                currentUserId: 'usr_self',
                notification
            })
        ).rejects.toThrow('send failed');
    });

    it('dismisses boops for a sender through the backend command', async () => {
        const { dismissBoopNotifications } =
            await import('./notificationActionService');

        await dismissBoopNotifications({
            currentUserId: 'usr_self',
            senderUserId: 'usr_sender'
        });

        expect(mocks.appNotificationBoopDismiss).toHaveBeenCalledWith({
            ownerUserId: 'usr_self',
            senderUserId: 'usr_sender'
        });
    });

    it('skips the boop dismiss command without a sender user id', async () => {
        const { dismissBoopNotifications } =
            await import('./notificationActionService');

        await dismissBoopNotifications({
            currentUserId: 'usr_self',
            senderUserId: ' '
        });

        expect(mocks.appNotificationBoopDismiss).not.toHaveBeenCalled();
    });

    it('throws the remote error for a failed notification response', async () => {
        mocks.appNotificationRespondAndExpire.mockResolvedValue(
            outcome({ status: 'remoteFailed', remoteError: 'response failed' })
        );
        const { sendNotificationButtonResponse } =
            await import('./notificationActionService');

        await expect(
            sendNotificationButtonResponse({
                currentUserId: 'usr_self',
                notification,
                response: { type: 'accept', data: 'payload' }
            })
        ).rejects.toThrow('response failed');

        expect(mocks.appNotificationRespondAndExpire).toHaveBeenCalledWith({
            ownerUserId: 'usr_self',
            target: {
                id: 'notif_target',
                version: 2,
                type: 'boop',
                senderUserId: 'usr_sender'
            },
            responseType: 'accept',
            responseData: 'payload'
        });
    });

    it('treats an already-resolved response as success', async () => {
        mocks.appNotificationRespondAndExpire.mockResolvedValue(
            outcome({
                status: 'alreadyResolved',
                expiredIds: ['notif_target'],
                remoteError: 'not found (404)'
            })
        );
        const { sendNotificationButtonResponse } =
            await import('./notificationActionService');

        await expect(
            sendNotificationButtonResponse({
                currentUserId: 'usr_self',
                notification,
                response: { type: 'accept', data: 'payload' }
            })
        ).resolves.toBeUndefined();
    });

    it('does not swallow a remote-ok-local-failed hide outcome', async () => {
        mocks.appNotificationHideAndExpire.mockResolvedValue(
            outcome({
                status: 'remoteOkLocalFailed',
                localError: 'database failed'
            })
        );
        const { hideRemoteAndExpireNotification } =
            await import('./notificationActionService');

        await expect(
            hideRemoteAndExpireNotification({
                currentUserId: 'usr_self',
                notification
            })
        ).rejects.toThrow('database failed');
    });

    it('resolves the invite request world name before delegating to the backend', async () => {
        const { acceptRequestInviteNotification } =
            await import('./notificationActionService');

        await acceptRequestInviteNotification({
            currentUserId: 'usr_self',
            notification,
            instanceId: 'wrld_1:1234',
            worldId: 'wrld_1'
        });

        expect(mocks.getWorlds).toHaveBeenCalledWith({}, 'wrld_1');
        expect(mocks.appNotificationRequestInviteAccept).toHaveBeenCalledWith({
            ownerUserId: 'usr_self',
            target: {
                id: 'notif_target',
                version: 2,
                type: 'boop',
                senderUserId: 'usr_sender'
            },
            instanceId: 'wrld_1:1234',
            worldId: 'wrld_1',
            worldName: 'World Name'
        });
    });

    it('skips the world lookup when the invite location is incomplete', async () => {
        const { acceptRequestInviteNotification } =
            await import('./notificationActionService');

        await acceptRequestInviteNotification({
            currentUserId: 'usr_self',
            notification,
            instanceId: '',
            worldId: 'wrld_1'
        });

        expect(mocks.getWorlds).not.toHaveBeenCalled();
        expect(mocks.appNotificationRequestInviteAccept).toHaveBeenCalledWith(
            expect.objectContaining({ instanceId: '', worldName: '' })
        );
    });

    it('wraps the photo invite response command in the upload timeout', async () => {
        const pending = deferred<NotificationActionOutcome>();
        mocks.appNotificationInviteResponseSend.mockReturnValue(
            pending.promise
        );
        const withUploadTimeout = vi.fn((promise: Promise<unknown>) => promise);
        const { sendInviteResponseNotification } =
            await import('./notificationActionService');

        const action = sendInviteResponseNotification({
            currentUserId: 'usr_self',
            notification,
            responseSlot: '1',
            imageData: 'base64data',
            withUploadTimeout
        });

        expect(withUploadTimeout).toHaveBeenCalledTimes(1);
        pending.resolve(outcome({ sentPhoto: true }));
        await expect(action).resolves.toEqual({ sentPhoto: true });
        expect(mocks.appNotificationInviteResponseSend).toHaveBeenCalledWith({
            ownerUserId: 'usr_self',
            target: {
                id: 'notif_target',
                version: 2,
                type: 'boop',
                senderUserId: 'usr_sender'
            },
            responseSlot: 1,
            imageData: 'base64data'
        });
    });

    it('sends a plain invite response without the upload timeout', async () => {
        const withUploadTimeout = vi.fn((promise: Promise<unknown>) => promise);
        const { sendInviteResponseNotification } =
            await import('./notificationActionService');

        await expect(
            sendInviteResponseNotification({
                currentUserId: 'usr_self',
                notification,
                responseSlot: 0,
                withUploadTimeout
            })
        ).resolves.toEqual({ sentPhoto: false });

        expect(withUploadTimeout).not.toHaveBeenCalled();
        expect(mocks.appNotificationInviteResponseSend).toHaveBeenCalledWith(
            expect.objectContaining({ responseSlot: 0, imageData: '' })
        );
    });

    it('accepts a friend request through the backend command and expires the notification', async () => {
        const accepted = deferred<{ status: string; targetUserId: string }>();
        mocks.appSocialFriendRequestAccept.mockReturnValue(accepted.promise);
        const { acceptFriendRequestNotification } =
            await import('./notificationActionService');

        const action = acceptFriendRequestNotification({
            currentUserId: 'usr_self',
            endpoint,
            notification
        });

        expect(mocks.expireNotification).not.toHaveBeenCalled();
        accepted.resolve({ status: 'applied', targetUserId: 'usr_sender' });
        await expect(action).resolves.toEqual({
            status: 'accepted',
            outcome: { status: 'applied', targetUserId: 'usr_sender' }
        });

        expect(mocks.appSocialFriendRequestAccept).toHaveBeenCalledWith({
            ownerUserId: 'usr_self',
            endpoint,
            notificationId: 'notif_target',
            targetUserId: 'usr_sender',
            targetDisplayName: 'Sender'
        });
        expect(mocks.expireNotification).toHaveBeenCalledWith({
            userId: 'usr_self',
            id: 'notif_target'
        });
    });

    it('treats a missing remote friend request as resolved locally', async () => {
        mocks.appSocialFriendRequestAccept.mockRejectedValue(
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

        expect(mocks.expireNotification).toHaveBeenCalledWith({
            userId: 'usr_self',
            id: 'notif_target'
        });
    });

    it('treats a backend (404) fallback message as resolved locally', async () => {
        mocks.appSocialFriendRequestAccept.mockRejectedValue(
            new Error('VRChat social mutation request failed (404)')
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

        expect(mocks.expireNotification).toHaveBeenCalledWith({
            userId: 'usr_self',
            id: 'notif_target'
        });
    });

    it('reports a remote-ok-local-failed outcome without swallowing it', async () => {
        mocks.appSocialFriendRequestAccept.mockResolvedValue({
            status: 'remoteOkLocalFailed',
            targetUserId: 'usr_sender',
            localError: 'database failed'
        });
        const { acceptFriendRequestNotification } =
            await import('./notificationActionService');

        await expect(
            acceptFriendRequestNotification({
                currentUserId: 'usr_self',
                endpoint,
                notification
            })
        ).resolves.toEqual({
            status: 'accepted',
            outcome: {
                status: 'remoteOkLocalFailed',
                targetUserId: 'usr_sender',
                localError: 'database failed'
            }
        });
        expect(mocks.expireNotification).toHaveBeenCalledWith({
            userId: 'usr_self',
            id: 'notif_target'
        });
    });

    it('rejects invalid action input before crossing the command boundary', async () => {
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
        expect(mocks.appNotificationBoopReply).not.toHaveBeenCalled();
        expect(mocks.appNotificationInviteResponseSend).not.toHaveBeenCalled();
    });
});
