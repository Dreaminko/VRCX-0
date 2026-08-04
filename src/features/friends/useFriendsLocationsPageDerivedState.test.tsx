// @vitest-environment jsdom

import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { FriendRecord } from '@/domain/friends/friendRosterTypes';

import { useFriendsLocationsPageDerivedState } from './useFriendsLocationsPageDerivedState';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

function friendAt(location: string): FriendRecord {
    return {
        id: 'usr_friend',
        displayName: 'Friend',
        tags: [],
        state: 'online',
        stateBucket: 'online',
        location,
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

describe('useFriendsLocationsPageDerivedState', () => {
    it('carries observed current-instance dwell epochs into searched card rows', () => {
        const location = 'wrld_test:123';
        const joinedAtMs = 1_700_000_000_000;
        const friend = friendAt(location);
        const { result } = renderHook(() =>
            useFriendsLocationsPageDerivedState({
                activeIds: [friend.id],
                activeSegment: 'online',
                collapsedGroups: new Set(),
                currentUserId: 'usr_self',
                currentUserSnapshot: null,
                deferredSearchQuery: 'Friend',
                density: 'compact',
                favoriteFriendGroups: [],
                friendsById: { [friend.id]: friend },
                gameState: {
                    currentLocation: location,
                    currentLocationPlayerIds: [friend.id],
                    currentLocationPlayers: [{ id: friend.id, joinedAtMs }],
                    isGameRunning: true
                },
                groupedFavoriteFriendIdsByGroupKey: {},
                localFriendFavoriteGroups: [],
                localFriendFavorites: {},
                offlineIds: [],
                onlineIds: [],
                remoteFavoriteFriendIds: [],
                rosterStatus: 'ready',
                scrollMetrics: {
                    width: 1000,
                    viewportHeight: 1000,
                    scrollTop: 0
                },
                showCurrentUserInSameInstance: true,
                showSameInstanceInOnline: true,
                sidebarFavoritePrefs: {
                    isDivideByGroup: false,
                    selectedGroups: [],
                    groupOrder: []
                },
                sidebarSortMethods: []
            })
        );

        const cardRow = result.current.visibleVirtualRows.find(
            (row) => row.type === 'cards'
        );
        expect(cardRow?.type).toBe('cards');
        if (cardRow?.type !== 'cards') {
            return;
        }
        expect(cardRow.friends[0]?.$location_at).toBe(joinedAtMs);
    });
});
