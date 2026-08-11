import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    appInstanceInviteBatch: vi.fn(),
    appNotificationInstanceInviteSend: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appInstanceInviteBatch: mocks.appInstanceInviteBatch,
        appNotificationInstanceInviteSend:
            mocks.appNotificationInstanceInviteSend
    }
}));

import {
    sendInviteToLocation,
    sendInvitesToLocation
} from './inviteDeliveryService';

describe('inviteDeliveryService', () => {
    beforeEach(() => {
        mocks.appInstanceInviteBatch.mockReset();
        mocks.appNotificationInstanceInviteSend.mockReset();
    });

    it('normalizes and sends one backend batch request', async () => {
        const result = {
            total: 2,
            succeeded: 1,
            failed: 1,
            items: []
        };
        mocks.appInstanceInviteBatch.mockResolvedValue(result);

        await expect(
            sendInvitesToLocation({
                receiverUserIds: [' usr_a ', '', 'usr_b'],
                location: ' wrld_test:12345 ',
                shortName: ' token ',
                worldName: ' Test World '
            })
        ).resolves.toBe(result);

        expect(mocks.appInstanceInviteBatch).toHaveBeenCalledOnce();
        expect(mocks.appInstanceInviteBatch).toHaveBeenCalledWith({
            receiverUserIds: ['usr_a', 'usr_b'],
            location: 'wrld_test:12345',
            shortName: 'token',
            worldName: 'Test World'
        });
    });

    it('delegates single-invite world resolution and optional fields to the backend', async () => {
        const result = {
            status: 'applied',
            expiredIds: [],
            sentPhoto: false,
            remoteError: null,
            localError: null
        };
        mocks.appNotificationInstanceInviteSend.mockResolvedValue(result);

        await expect(
            sendInviteToLocation({
                receiverUserId: ' usr_receiver ',
                instanceId: ' wrld_test:12345 ',
                worldId: ' wrld_test ',
                messageSlot: 3,
                rsvp: true
            })
        ).resolves.toBe(result);

        expect(mocks.appNotificationInstanceInviteSend).toHaveBeenCalledWith({
            receiverUserId: 'usr_receiver',
            instanceId: 'wrld_test:12345',
            worldId: 'wrld_test',
            worldName: '',
            messageSlot: 3,
            imageData: '',
            rsvp: true
        });
    });
});
