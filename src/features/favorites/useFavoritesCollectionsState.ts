import { useEffect, useMemo, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';

import { useLocalWorldFavorites } from '@/components/favorites/useLocalWorldFavorites';
import { useKnownUserFacts } from '@/lib/useKnownUser';
import avatarLocalRepository from '@/repositories/avatarLocalRepository';
import { useFavoriteStore } from '@/state/favoriteStore';
import { useFriendRosterStore } from '@/state/friendRosterStore';

import {
    buildFavoriteAvatarDetailIds,
    buildFavoriteAvatarTags,
    buildFavoriteFriendFactIds,
    buildFavoriteRemoteGroupEntityIds,
    selectFavoritesCollectionsState
} from './favoritesCollectionsState';
import type { FavoriteKind, FavoriteSource } from './favoritesTypes';
import { useAvatarDetailFallbacks } from './useAvatarDetailFallbacks';
import { useFavoriteRemoteDetails } from './useFavoriteRemoteDetails';
import { useWorldDetailFallbacks } from './useWorldDetailFallbacks';

function selectRequestedRemoteEntityIds({
    favoriteAvatarIds,
    favoriteWorldIds,
    kind,
    loadAllRemoteDetails,
    selectedRemoteEntityIds,
    selectedSource
}: {
    favoriteAvatarIds: string[];
    favoriteWorldIds: string[];
    kind: FavoriteKind;
    loadAllRemoteDetails: boolean;
    selectedRemoteEntityIds: string[];
    selectedSource: FavoriteSource;
}): string[] {
    if (kind === 'avatar') {
        return favoriteAvatarIds;
    }
    if (kind !== 'world') {
        return [];
    }
    if (loadAllRemoteDetails) {
        return favoriteWorldIds;
    }
    if (selectedSource === 'remote') {
        return selectedRemoteEntityIds;
    }
    return [];
}

export function useFavoritesCollectionsState({
    currentEndpoint,
    currentUserId,
    kind,
    loadAllRemoteDetails,
    selectedGroupKey,
    selectedSource
}: {
    currentEndpoint: string;
    currentUserId: string;
    kind: FavoriteKind;
    loadAllRemoteDetails: boolean;
    selectedGroupKey: string;
    selectedSource: FavoriteSource;
}) {
    const favoriteSelector = useMemo(
        () => selectFavoritesCollectionsState(kind),
        [kind]
    );
    const favoriteState = useFavoriteStore(useShallow(favoriteSelector));
    const localWorldFavorites = useLocalWorldFavorites(kind === 'world');
    const friendsById = useFriendRosterStore((state) => state.friendsById);
    const [avatarHistoryLoading, setAvatarHistoryLoading] = useState(false);
    const [avatarHistory, setAvatarHistory] = useState<unknown[]>([]);
    const friendsMap = useMemo(
        () => new Map(Object.entries(friendsById || {})),
        [friendsById]
    );
    const favoriteFriendFactIds = useMemo(
        () =>
            buildFavoriteFriendFactIds({
                groupedFavoriteFriendIdsByGroupKey:
                    favoriteState.groupedFavoriteFriendIdsByGroupKey,
                kind,
                localFriendFavorites: favoriteState.localFriendFavorites
            }),
        [
            favoriteState.groupedFavoriteFriendIdsByGroupKey,
            favoriteState.localFriendFavorites,
            kind
        ]
    );
    const knownFavoriteUsersById = useKnownUserFacts(favoriteFriendFactIds, {
        endpoint: currentEndpoint
    });
    const avatarTags = useMemo(
        () =>
            buildFavoriteAvatarTags({
                kind,
                remoteFavoritesById: favoriteState.remoteFavoritesById
            }),
        [favoriteState.remoteFavoritesById, kind]
    );
    const selectedRemoteEntityIds = useMemo(
        () =>
            buildFavoriteRemoteGroupEntityIds({
                groupKey: selectedGroupKey,
                kind,
                remoteFavoritesById: favoriteState.remoteFavoritesById
            }),
        [favoriteState.remoteFavoritesById, kind, selectedGroupKey]
    );
    const requestedRemoteEntityIds = selectRequestedRemoteEntityIds({
        favoriteAvatarIds: favoriteState.favoriteAvatarIds,
        favoriteWorldIds: favoriteState.favoriteWorldIds,
        kind,
        loadAllRemoteDetails,
        selectedRemoteEntityIds,
        selectedSource
    });
    const remoteEntityDetails = useFavoriteRemoteDetails({
        type: kind === 'avatar' ? 'avatar' : 'world',
        favoriteIds:
            kind === 'world'
                ? favoriteState.favoriteWorldIds
                : kind === 'avatar'
                  ? favoriteState.favoriteAvatarIds
                  : [],
        requestedIds: requestedRemoteEntityIds,
        avatarTags,
        cacheKey: favoriteState.favoriteLastLoadedAt || '',
        enabled:
            kind !== 'friend' &&
            favoriteState.favoriteLoadStatus === 'ready' &&
            requestedRemoteEntityIds.length > 0
    });
    const requestedWorldIds = useMemo(() => {
        if (kind !== 'world') {
            return [];
        }
        const worldIds = new Set(requestedRemoteEntityIds);
        let localGroups: string[][] = [];
        if (loadAllRemoteDetails) {
            localGroups = Object.values(localWorldFavorites.favoritesByGroup);
        } else if (selectedSource === 'local') {
            localGroups = [
                localWorldFavorites.favoritesByGroup[selectedGroupKey] || []
            ];
        }
        for (const ids of localGroups) {
            for (const worldId of ids) {
                const normalizedWorldId = worldId.trim();
                if (normalizedWorldId) {
                    worldIds.add(normalizedWorldId);
                }
            }
        }
        return Array.from(worldIds);
    }, [
        kind,
        loadAllRemoteDetails,
        localWorldFavorites.favoritesByGroup,
        requestedRemoteEntityIds,
        selectedGroupKey,
        selectedSource
    ]);
    const worldDetailFallbacksById = useWorldDetailFallbacks({
        worldIds: requestedWorldIds,
        kind,
        remoteEntityDetailsData: remoteEntityDetails.data,
        remoteEntityDetailsStatus:
            requestedRemoteEntityIds.length === 0
                ? 'ready'
                : remoteEntityDetails.status
    });
    const requestedAvatarIds = useMemo(() => {
        return buildFavoriteAvatarDetailIds({
            favoriteAvatarIds: favoriteState.favoriteAvatarIds,
            kind,
            localAvatarFavorites: favoriteState.localAvatarFavorites
        });
    }, [
        favoriteState.favoriteAvatarIds,
        favoriteState.localAvatarFavorites,
        kind
    ]);
    const avatarDetailFallbacksById = useAvatarDetailFallbacks({
        avatarIds: requestedAvatarIds,
        kind,
        remoteEntityDetailsData: remoteEntityDetails.data,
        remoteEntityDetailsStatus:
            favoriteState.favoriteAvatarIds.length === 0
                ? 'ready'
                : remoteEntityDetails.status
    });
    const worldAvailabilityById = remoteEntityDetails.availabilityById;

    useEffect(() => {
        let active = true;
        if (kind !== 'avatar' || !currentUserId) {
            setAvatarHistory([]);
            return () => {
                active = false;
            };
        }
        setAvatarHistoryLoading(true);
        avatarLocalRepository
            .getAvatarHistory(currentUserId, 100)
            .then((rows) => {
                if (active) {
                    setAvatarHistory(rows);
                }
            })
            .catch(() => {
                if (active) {
                    setAvatarHistory([]);
                }
            })
            .finally(() => {
                if (active) {
                    setAvatarHistoryLoading(false);
                }
            });
        return () => {
            active = false;
        };
    }, [currentUserId, kind]);

    return {
        avatarHistory,
        avatarHistoryLoading,
        favoriteDetail: favoriteState.favoriteDetail,
        favoriteLoadStatus: favoriteState.favoriteLoadStatus,
        remoteEntityDetails,
        setAvatarHistory,
        setAvatarHistoryLoading,
        actionInputs: {
            avatarHistoryLoading,
            friendsById,
            friendsMap,
            reloadLocalWorldFavorites: localWorldFavorites.reload,
            setAvatarHistory,
            setAvatarHistoryLoading
        },
        viewDataInputs: {
            avatarHistory,
            favoriteAvatarGroups: favoriteState.favoriteAvatarGroups,
            favoriteFriendGroups: favoriteState.favoriteFriendGroups,
            favoriteWorldGroups: favoriteState.favoriteWorldGroups,
            favoritesSortOrder: favoriteState.favoritesSortOrder,
            friendsById,
            groupedFavoriteFriendIdsByGroupKey:
                favoriteState.groupedFavoriteFriendIdsByGroupKey,
            knownUsersById: knownFavoriteUsersById,
            localAvatarFavoriteGroups: favoriteState.localAvatarFavoriteGroups,
            localAvatarFavorites: favoriteState.localAvatarFavorites,
            localFriendFavoriteGroups: favoriteState.localFriendFavoriteGroups,
            localFriendFavorites: favoriteState.localFriendFavorites,
            localWorldFavoriteGroups: localWorldFavorites.groupNames,
            localWorldFavorites: localWorldFavorites.favoritesByGroup,
            remoteEntityDetails,
            remoteFavoritesById: favoriteState.remoteFavoritesById,
            worldDetailFallbacksById,
            avatarDetailFallbacksById,
            worldAvailabilityById
        }
    };
}
