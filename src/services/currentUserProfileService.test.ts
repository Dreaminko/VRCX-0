import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    addCurrentUserTags: vi.fn(),
    recordUserProfile: vi.fn(),
    removeCurrentUserTags: vi.fn(),
    updateCurrentUser: vi.fn()
}));

vi.mock('@/repositories/userProfileRepository', () => ({
    default: {
        addCurrentUserTags: mocks.addCurrentUserTags,
        removeCurrentUserTags: mocks.removeCurrentUserTags,
        updateCurrentUser: mocks.updateCurrentUser
    }
}));

vi.mock('@/services/userFactAccessService', () => ({
    recordUserProfile: mocks.recordUserProfile
}));

import currentUserProfileService from './currentUserProfileService';

describe('currentUserProfileService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it.each([
        [
            'updateCurrentUser',
            mocks.updateCurrentUser,
            () =>
                currentUserProfileService.updateCurrentUser({
                    userId: 'usr_self',
                    params: { status: 'busy' }
                })
        ],
        [
            'addCurrentUserTags',
            mocks.addCurrentUserTags,
            () =>
                currentUserProfileService.addCurrentUserTags({
                    userId: 'usr_self',
                    tags: ['language_en']
                })
        ],
        [
            'removeCurrentUserTags',
            mocks.removeCurrentUserTags,
            () =>
                currentUserProfileService.removeCurrentUserTags({
                    userId: 'usr_self',
                    tags: ['language_en']
                })
        ]
    ] as const)(
        'records the resolved profile after %s',
        async (_, call, run) => {
            const profile = {
                id: 'usr_self',
                displayName: 'Current User'
            };
            call.mockResolvedValueOnce(profile);

            await expect(run()).resolves.toBe(profile);

            expect(mocks.recordUserProfile).toHaveBeenCalledWith(profile, {
                endpoint: 'https://api.vrchat.cloud/api/1',
                source: 'currentUser',
                isCurrentUser: true
            });
            expect(call.mock.invocationCallOrder[0]).toBeLessThan(
                mocks.recordUserProfile.mock.invocationCallOrder[0]
            );
        }
    );

    it('does not record a profile when the repository mutation fails', async () => {
        const error = new Error('update failed');
        mocks.updateCurrentUser.mockRejectedValueOnce(error);

        await expect(
            currentUserProfileService.updateCurrentUser({
                userId: 'usr_self',
                params: { status: 'busy' }
            })
        ).rejects.toBe(error);

        expect(mocks.recordUserProfile).not.toHaveBeenCalled();
    });
});
