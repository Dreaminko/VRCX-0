import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    appInstanceInviteBatch: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appInstanceInviteBatch: mocks.appInstanceInviteBatch
    }
}));

import { sendInvitesToLocation } from './inviteDeliveryService';

describe('inviteDeliveryService', () => {
    beforeEach(() => {
        mocks.appInstanceInviteBatch.mockReset();
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
});
