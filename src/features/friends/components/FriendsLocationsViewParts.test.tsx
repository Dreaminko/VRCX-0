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

vi.mock('./FriendLocationCard', () => ({
    FriendLocationCard: ({
        location,
        capabilities
    }: {
        location?: { instanceEpoch?: unknown };
        capabilities?: {
            useLocation?: boolean;
            sendInvite?: boolean;
            requestInvite?: boolean;
            boop?: boolean;
        };
    }) => (
        <span
            data-instance-epoch={String(location?.instanceEpoch ?? '')}
            data-can-use-location={String(Boolean(capabilities?.useLocation))}
            data-can-send-invite={String(Boolean(capabilities?.sendInvite))}
            data-can-request-invite={String(
                Boolean(capabilities?.requestInvite)
            )}
            data-can-boop={String(Boolean(capabilities?.boop))}
        />
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
                onLaunchLocation={vi.fn()}
                onSelfInviteLocation={vi.fn()}
                onSendInvite={vi.fn()}
                onRequestInvite={vi.fn()}
                onSendBoop={vi.fn()}
            />
        );

        expect(html).toContain('data-instance-epoch="1700000000000"');
        expect(html).toContain('data-can-use-location="true"');
        expect(html).toContain('data-can-send-invite="true"');
        expect(html).toContain('data-can-request-invite="true"');
        expect(html).toContain('data-can-boop="true"');
    });

    it('disables every social and location action for the current user', () => {
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
                currentUserId={friend.id}
                densityConfig={getFriendsLocationsDensityConfig('compact')}
                canUseFriendLocation={() => true}
                canSendInvite
                canBoop
                onOpenUser={vi.fn()}
                onOpenWorld={vi.fn()}
                onLaunchLocation={vi.fn()}
                onSelfInviteLocation={vi.fn()}
                onSendInvite={vi.fn()}
                onRequestInvite={vi.fn()}
                onSendBoop={vi.fn()}
            />
        );

        expect(html).toContain('data-can-use-location="false"');
        expect(html).toContain('data-can-send-invite="false"');
        expect(html).toContain('data-can-request-invite="false"');
        expect(html).toContain('data-can-boop="false"');
    });
});
