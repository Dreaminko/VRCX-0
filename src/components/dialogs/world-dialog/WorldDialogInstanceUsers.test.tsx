// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getSharedSameInstanceFallbackJoinTimes } from '@/components/sidebar/friends-sidebar/friendsSidebarModel';

type QueryOptions = {
    enabled?: boolean;
    queryFn: () => Promise<unknown>;
};

type RuntimeStoreState = {
    auth: {
        currentUserEndpoint: string;
        currentUserSnapshot: null;
    };
    gameState: {
        isGameRunning: boolean;
    };
};

const mocks = vi.hoisted(() => ({
    getUserProfile: vi.fn(() => Promise.resolve({})),
    knownCreatorUser: null as Record<string, unknown> | null,
    queryData: null as Record<string, unknown> | null
}));

vi.mock('@tanstack/react-query', async (importOriginal) => {
    const actual =
        await importOriginal<typeof import('@tanstack/react-query')>();
    const { useEffect } = await import('react');
    return {
        ...actual,
        useQuery: (options: QueryOptions) => {
            const { enabled, queryFn } = options;
            useEffect(() => {
                if (enabled) {
                    void queryFn();
                }
            }, [enabled, queryFn]);
            return { data: mocks.queryData };
        }
    };
});

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock(
    '@/components/sidebar/friends-sidebar/friendsSidebarModel',
    async (importOriginal) => ({
        ...(await importOriginal<
            typeof import('@/components/sidebar/friends-sidebar/friendsSidebarModel')
        >()),
        resolveSidebarStatusDotClassName: () => ''
    })
);

vi.mock('@/components/UserDetailTile', () => ({
    UserDetailTile: ({
        displayName,
        imageUrl,
        namePrefix,
        subline
    }: {
        displayName: unknown;
        imageUrl?: string;
        namePrefix?: ReactNode;
        subline?: ReactNode;
    }) => (
        <div
            data-testid="user-detail-tile"
            data-display-name={
                typeof displayName === 'string' ? displayName : ''
            }
            data-image-url={imageUrl}
        >
            {namePrefix}
            {subline}
        </div>
    )
}));

vi.mock('@/components/sidebar/friends-sidebar/FriendsSidebarLocation', () => ({
    FriendInstanceTimer: ({ epoch }: { epoch?: unknown }) => (
        <span data-testid="instance-timer" data-epoch={String(epoch)} />
    )
}));

vi.mock('@/lib/useKnownUser', () => ({
    useKnownUserFact: () => mocks.knownCreatorUser
}));

vi.mock('@/repositories/userProfileRepository', () => ({
    default: {
        getUserProfile: mocks.getUserProfile
    }
}));

vi.mock('@/services/dialogService', () => ({
    openUserDialog: vi.fn()
}));

vi.mock('@/services/entityMediaService', () => ({
    userImage: (user: unknown) => {
        if (
            user &&
            typeof user === 'object' &&
            'thumbnailImageUrl' in user &&
            typeof user.thumbnailImageUrl === 'string'
        ) {
            return user.thumbnailImageUrl;
        }
        return '';
    }
}));

vi.mock('@/shared/utils/userStatus', () => ({
    userStatusLabel: (
        user: unknown,
        t: (key: string, options?: { defaultValue: string }) => string
    ) => {
        const state =
            user && typeof user === 'object' && 'state' in user
                ? String(user.state)
                : '';
        return state ? t(`dialog.user.status.${state}`) : '';
    }
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: <T,>(selector: (state: RuntimeStoreState) => T): T =>
        selector({
            auth: {
                currentUserEndpoint: 'https://api.vrchat.cloud',
                currentUserSnapshot: null
            },
            gameState: {
                isGameRunning: false
            }
        })
}));

vi.mock('@/ui/shadcn/spinner', () => ({
    Spinner: () => null
}));

import { InstanceUserTiles } from './WorldDialogInstanceUsers';

describe('InstanceUserTiles', () => {
    afterEach(cleanup);

    beforeEach(() => {
        vi.clearAllMocks();
        mocks.knownCreatorUser = null;
        mocks.queryData = null;
        getSharedSameInstanceFallbackJoinTimes().clear();
    });

    it('fetches an unresolved non-friend instance creator profile', async () => {
        mocks.queryData = {
            id: 'usr_non_friend_owner',
            displayName: 'Remote Owner',
            thumbnailImageUrl: 'https://images.example/remote-owner.png'
        };

        render(
            <InstanceUserTiles
                instance={{ creatorUserId: 'usr_non_friend_owner' }}
            />
        );

        await waitFor(() => {
            expect(mocks.getUserProfile).toHaveBeenCalledWith({
                userId: 'usr_non_friend_owner'
            });
        });
        const tile = screen.getByTestId('user-detail-tile');
        expect(tile.getAttribute('data-display-name')).toBe('Remote Owner');
        expect(tile.getAttribute('data-image-url')).toBe(
            'https://images.example/remote-owner.png'
        );
    });

    it('does not refetch a creator whose known fact has display media', () => {
        mocks.knownCreatorUser = {
            id: 'usr_friend_owner',
            displayName: 'Friend Owner',
            thumbnailImageUrl: 'https://images.example/friend-owner.png'
        };

        render(
            <InstanceUserTiles
                instance={{ creatorUserId: 'usr_friend_owner' }}
            />
        );

        expect(mocks.getUserProfile).not.toHaveBeenCalled();
    });

    it('keeps the creator while filtering other non-friends', () => {
        render(
            <InstanceUserTiles
                instance={{
                    creatorUserId: 'usr_non_friend_owner',
                    creatorUser: {
                        statusDescription: 'Owner signature',
                        $location_at: 1_700_000_000_000
                    },
                    users: [
                        { id: 'usr_self', displayName: 'Self' },
                        { id: 'usr_friend', displayName: 'Friend' },
                        { id: 'usr_non_friend', displayName: 'Non-friend' }
                    ]
                }}
                visibleUserIds={new Set(['usr_self', 'usr_friend'])}
                showInstanceDuration
            />
        );

        expect(
            screen
                .getAllByTestId('user-detail-tile')
                .map((tile) => tile.getAttribute('data-display-name'))
        ).toEqual(['usr_non_friend_owner', 'Self', 'Friend']);
        expect(screen.getByText('Owner signature')).toBeTruthy();
        expect(
            screen
                .getByText('Owner signature')
                .closest('[data-testid="user-detail-tile"]')
                ?.querySelector('[data-testid="instance-timer"]')
        ).toBeNull();
        expect(
            screen.getByLabelText('dialog.world.instances.instance_creator')
        ).toBeTruthy();
    });

    it('shows the timer for a friend creator', () => {
        mocks.knownCreatorUser = {
            id: 'usr_friend_owner',
            displayName: 'Friend Owner',
            isFriend: true,
            statusDescription: 'Friend signature',
            thumbnailImageUrl: 'https://images.example/friend-owner.png'
        };

        render(
            <InstanceUserTiles
                instance={{
                    creatorUserId: 'usr_friend_owner',
                    users: [
                        {
                            id: 'usr_friend_owner',
                            ref: {
                                $location_at: 1_700_000_000_000
                            }
                        }
                    ]
                }}
                visibleUserIds={new Set(['usr_friend_owner'])}
                showInstanceDuration
            />
        );

        expect(screen.getByTestId('instance-timer').dataset.epoch).toBe(
            '1700000000000'
        );
        const tiles = screen.getAllByTestId('user-detail-tile');
        expect(tiles).toHaveLength(1);
        expect(tiles[0]?.getAttribute('data-display-name')).toBe(
            'Friend Owner'
        );
        expect(screen.queryByText('Friend signature')).toBeNull();
    });

    it('shows the localized status when a non-friend creator has no signature', () => {
        render(
            <InstanceUserTiles
                instance={{
                    creatorUserId: 'usr_non_friend_owner',
                    creatorUser: { state: 'active' }
                }}
                visibleUserIds={new Set()}
                showInstanceDuration
            />
        );

        expect(screen.getByText('dialog.user.status.active')).toBeTruthy();
        expect(screen.queryByTestId('instance-timer')).toBeNull();
    });

    it('shows the instance timer instead of the status signature', () => {
        render(
            <InstanceUserTiles
                instance={{
                    users: [
                        {
                            id: 'usr_friend',
                            displayName: 'Friend',
                            statusDescription: 'World hopping',
                            $location_at: 1_700_000_000_000
                        }
                    ]
                }}
                showInstanceDuration
            />
        );

        expect(screen.getByTestId('instance-timer').dataset.epoch).toBe(
            '1700000000000'
        );
        expect(screen.queryByText('World hopping')).toBeNull();
    });

    it('reuses the sidebar fallback when the roster has no join time', () => {
        getSharedSameInstanceFallbackJoinTimes().set(
            'wrld_test:123:usr_friend',
            1_700_000_000_000
        );

        render(
            <InstanceUserTiles
                instance={{
                    location: 'wrld_test:123',
                    users: [{ id: 'usr_friend', displayName: 'Friend' }]
                }}
                showInstanceDuration
            />
        );

        expect(screen.getByTestId('instance-timer').dataset.epoch).toBe(
            '1700000000000'
        );
    });

    it('keeps the sidebar fallback when creator profile data arrives later', () => {
        mocks.knownCreatorUser = {
            id: 'usr_friend_owner',
            displayName: 'Friend Owner',
            isFriend: true,
            $location_at: 1_700_000_030_000
        };
        getSharedSameInstanceFallbackJoinTimes().set(
            'wrld_test:123:usr_friend_owner',
            1_700_000_000_000
        );

        render(
            <InstanceUserTiles
                instance={{
                    location: 'wrld_test:123',
                    creatorUserId: 'usr_friend_owner'
                }}
                visibleUserIds={new Set(['usr_friend_owner'])}
                showInstanceDuration
            />
        );

        expect(screen.getByTestId('instance-timer').dataset.epoch).toBe(
            '1700000000000'
        );
    });
});
