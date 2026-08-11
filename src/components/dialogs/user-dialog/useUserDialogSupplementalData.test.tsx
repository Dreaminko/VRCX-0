// @vitest-environment jsdom

import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getFriendLogHistory: vi.fn(),
    getRepresentedGroup: vi.fn(),
    getUserStats: vi.fn()
}));

vi.mock('@/repositories/friendLogHistoryRepository', () => ({
    default: { getFriendLogHistory: mocks.getFriendLogHistory }
}));
vi.mock('@/repositories/gameLogRepository', () => ({
    default: { getUserStats: mocks.getUserStats }
}));
vi.mock('@/repositories/userProfileRepository', () => ({
    default: {
        getRepresentedGroup: mocks.getRepresentedGroup
    }
}));

import { useUserDialogSupplementalData } from './useUserDialogSupplementalData';

function input(profile: Record<string, unknown>) {
    return {
        activeUserTargetRef: {
            current: {
                endpoint: 'https://api.example.test',
                userId: 'usr_target'
            }
        },
        currentEndpoint: 'https://api.example.test',
        currentGameDestination: '',
        currentGameLocation: 'wrld_current:1',
        currentSnapshotLocation: '',
        currentUserId: 'usr_self',
        currentUserSnapshot: {
            id: 'usr_self',
            hasSharedConnectionsOptOut: false
        },
        isTargetCurrentUser: false,
        normalizedUserId: 'usr_target',
        openNonce: 1,
        profile,
        reloadToken: 0,
        targetKey: 'https://api.example.test::usr_target'
    };
}

describe('useUserDialogSupplementalData', () => {
    beforeEach(() => {
        for (const mock of Object.values(mocks)) {
            mock.mockReset();
            mock.mockResolvedValue([]);
        }
        mocks.getUserStats.mockResolvedValue({});
        mocks.getRepresentedGroup.mockResolvedValue(null);
    });

    it('does not refetch id-based supplemental rows for a display-only profile merge', async () => {
        mocks.getUserStats.mockResolvedValue({
            previousDisplayNames: [
                {
                    displayName: 'Merged Name',
                    updated_at: '2026-08-10T00:00:00.000Z'
                },
                {
                    displayName: 'Older Name',
                    updated_at: '2026-08-09T00:00:00.000Z'
                }
            ]
        });
        const baseProfile: Record<string, unknown> = {
            id: 'usr_target',
            displayName: 'Initial Name',
            location: 'wrld_other:2'
        };
        const { rerender, result } = renderHook(
            ({ profile }) => useUserDialogSupplementalData(input(profile)),
            { initialProps: { profile: baseProfile } }
        );

        await waitFor(() => {
            expect(mocks.getUserStats).toHaveBeenCalledTimes(1);
            expect(mocks.getFriendLogHistory).toHaveBeenCalledTimes(1);
            expect(result.current.userStats.previousDisplayNames).toEqual([
                {
                    displayName: 'Merged Name',
                    updated_at: '2026-08-10T00:00:00.000Z'
                },
                {
                    displayName: 'Older Name',
                    updated_at: '2026-08-09T00:00:00.000Z'
                }
            ]);
        });

        rerender({
            profile: {
                ...baseProfile,
                displayName: 'Merged Name',
                currentAvatarImageUrl: 'https://example.test/avatar.png'
            }
        });

        await waitFor(() => {
            expect(result.current.userStats.previousDisplayNames).toEqual([
                {
                    displayName: 'Older Name',
                    updated_at: '2026-08-09T00:00:00.000Z'
                }
            ]);
        });
        expect(mocks.getUserStats).toHaveBeenCalledTimes(1);
        expect(mocks.getFriendLogHistory).toHaveBeenCalledTimes(1);

        rerender({
            profile: {
                ...baseProfile,
                displayName: 'Merged Name',
                location: 'wrld_current:1'
            }
        });

        await waitFor(() => {
            expect(mocks.getUserStats).toHaveBeenCalledTimes(2);
        });
        expect(mocks.getFriendLogHistory).toHaveBeenCalledTimes(1);
    });
});
