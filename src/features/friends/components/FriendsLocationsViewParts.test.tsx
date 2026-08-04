import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

import type { FriendRecord } from '@/domain/friends/friendRosterTypes';
import { getFriendsLocationsDensityConfig } from '@/features/friends/friendsLocationsDensity';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock('@/components/Location', () => ({
    Location: () => <span />
}));

vi.mock('@/components/friends/FriendLocationCard', () => ({
    FriendLocationCard: ({ instanceEpoch }: { instanceEpoch?: unknown }) => (
        <span data-instance-epoch={String(instanceEpoch ?? '')} />
    )
}));

import { FriendsLocationCardItem } from './FriendsLocationsViewParts';

function friendAt(location: string): FriendRecord {
    return {
        id: 'usr_friend',
        displayName: 'Friend',
        tags: [],
        state: 'online',
        stateBucket: 'online',
        location,
        $location_at: 1_700_000_000_000,
        $trustLevel: '',
        $friendNumber: 0,
        $trustClass: '',
        $trustSortNum: 0,
        $isModerator: false,
        $isTroll: false,
        $isProbableTroll: false,
        $platform: ''
    };
}

describe('FriendsLocationCardItem', () => {
    it('passes the room dwell epoch to the shared card timer', () => {
        const location = 'wrld_test:123';
        const friend = friendAt(location);
        const html = renderToStaticMarkup(
            <FriendsLocationCardItem
                section={{
                    key: `instance:${location}`,
                    title: 'World',
                    description: '',
                    friends: [friend],
                    worldId: 'wrld_test',
                    groupId: '',
                    rawLocation: location
                }}
                friend={friend}
                currentUserId="usr_self"
                densityConfig={getFriendsLocationsDensityConfig('compact')}
                canUseFriendLocation={() => true}
                canSendInvite
                canBoop
                onOpenUser={vi.fn()}
                onOpenWorld={vi.fn()}
                onOpenGroup={vi.fn()}
                onLaunchLocation={vi.fn()}
                onSelfInviteLocation={vi.fn()}
                onSendInvite={vi.fn()}
                onRequestInvite={vi.fn()}
                onSendBoop={vi.fn()}
            />
        );

        expect(html).toContain('data-instance-epoch="1700000000000"');
    });
});
