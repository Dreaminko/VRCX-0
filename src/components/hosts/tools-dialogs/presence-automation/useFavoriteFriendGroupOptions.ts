import { useMemo } from 'react';

import { useFavoriteStore } from '@/state/favoriteStore';

import {
    createGroupOptions,
    type PresenceOption
} from './presenceAutomationDialogUtils';

export function useFavoriteFriendGroupOptions(): PresenceOption[] {
    const favoriteFriendGroups = useFavoriteStore(
        (state) => state.favoriteFriendGroups
    );
    const localFriendFavoriteGroups = useFavoriteStore(
        (state) => state.localFriendFavoriteGroups
    );

    return useMemo(
        () =>
            createGroupOptions({
                remoteGroups: favoriteFriendGroups,
                localGroups: localFriendFavoriteGroups
            }),
        [favoriteFriendGroups, localFriendFavoriteGroups]
    );
}
