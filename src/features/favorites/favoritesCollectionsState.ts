import type { FavoriteStore } from '@/state/favoriteStore';

import { normalizeFavoriteEntityId as normalizeEntityId } from './favoritesItems';
import type { FavoriteKind } from './favoritesTypes';

const EMPTY_ARRAY: [] = [];
const EMPTY_OBJECT: Record<string, never> = {};

type FavoritesCollectionsStoreSlice = Pick<
    FavoriteStore,
    | 'detail'
    | 'favoriteAvatarGroups'
    | 'favoriteAvatarIds'
    | 'favoriteFriendGroups'
    | 'favoriteWorldGroups'
    | 'favoriteWorldIds'
    | 'favoritesSortOrder'
    | 'groupedFavoriteFriendIdsByGroupKey'
    | 'localAvatarFavoriteGroups'
    | 'localAvatarFavorites'
    | 'localFriendFavoriteGroups'
    | 'localFriendFavorites'
    | 'localWorldDetailsById'
    | 'localWorldFavoriteGroups'
    | 'localWorldFavorites'
    | 'remoteFavoritesById'
> & {
    loadStatus: string;
};

type FavoriteAvatarTagSource = {
    tags?: unknown[];
    type?: unknown;
};

function addNormalizedFavoriteIds(
    ids: Set<string>,
    idsByGroupKey: Record<string, unknown>
) {
    for (const groupIds of Object.values(idsByGroupKey)) {
        for (const favoriteId of Array.isArray(groupIds) ? groupIds : []) {
            const normalizedId = normalizeEntityId(favoriteId);
            if (normalizedId) {
                ids.add(normalizedId);
            }
        }
    }
}

export function buildFavoriteFriendFactIds({
    groupedFavoriteFriendIdsByGroupKey = EMPTY_OBJECT,
    kind,
    localFriendFavorites = EMPTY_OBJECT
}: {
    groupedFavoriteFriendIdsByGroupKey?: Record<string, unknown>;
    kind: FavoriteKind;
    localFriendFavorites?: Record<string, unknown>;
}) {
    if (kind !== 'friend') {
        return [];
    }

    const ids = new Set<string>();
    addNormalizedFavoriteIds(ids, groupedFavoriteFriendIdsByGroupKey);
    addNormalizedFavoriteIds(ids, localFriendFavorites);
    return Array.from(ids);
}

export function buildFavoriteAvatarDetailIds({
    favoriteAvatarIds = EMPTY_ARRAY,
    kind,
    localAvatarFavorites = EMPTY_OBJECT
}: {
    favoriteAvatarIds?: unknown[];
    kind: FavoriteKind;
    localAvatarFavorites?: Record<string, unknown>;
}) {
    if (kind !== 'avatar') {
        return [];
    }

    const ids = new Set<string>();
    for (const avatarId of favoriteAvatarIds) {
        const normalizedId = normalizeEntityId(avatarId);
        if (normalizedId) {
            ids.add(normalizedId);
        }
    }
    addNormalizedFavoriteIds(ids, localAvatarFavorites);
    return Array.from(ids);
}

export function buildFavoriteAvatarTags({
    kind,
    remoteFavoritesById = EMPTY_OBJECT
}: {
    kind: FavoriteKind;
    remoteFavoritesById?: Record<string, unknown>;
}) {
    if (kind !== 'avatar') {
        return [];
    }

    return Array.from(
        new Set(
            Object.values(remoteFavoritesById)
                .filter(
                    (favorite): favorite is FavoriteAvatarTagSource =>
                        Boolean(favorite && typeof favorite === 'object') &&
                        (favorite as FavoriteAvatarTagSource).type === 'avatar'
                )
                .map((favorite) =>
                    Array.isArray(favorite.tags) &&
                    typeof favorite.tags[0] === 'string'
                        ? favorite.tags[0].trim()
                        : ''
                )
                .filter(Boolean)
        )
    );
}

export function selectFavoritesCollectionsState(kind: FavoriteKind) {
    return (state: FavoritesCollectionsStoreSlice) => {
        const isFriend = kind === 'friend';
        const isAvatar = kind === 'avatar';
        const isWorld = kind === 'world';

        return {
            favoriteLoadStatus: state.loadStatus,
            favoriteDetail: state.detail,
            favoritesSortOrder: state.favoritesSortOrder,
            remoteFavoritesById:
                isAvatar || isWorld ? state.remoteFavoritesById : EMPTY_OBJECT,
            favoriteFriendGroups: isFriend
                ? state.favoriteFriendGroups
                : EMPTY_ARRAY,
            favoriteWorldGroups: isWorld
                ? state.favoriteWorldGroups
                : EMPTY_ARRAY,
            favoriteAvatarGroups: isAvatar
                ? state.favoriteAvatarGroups
                : EMPTY_ARRAY,
            groupedFavoriteFriendIdsByGroupKey: isFriend
                ? state.groupedFavoriteFriendIdsByGroupKey
                : EMPTY_OBJECT,
            localWorldFavorites: isWorld
                ? state.localWorldFavorites
                : EMPTY_OBJECT,
            localAvatarFavorites: isAvatar
                ? state.localAvatarFavorites
                : EMPTY_OBJECT,
            localFriendFavorites: isFriend
                ? state.localFriendFavorites
                : EMPTY_OBJECT,
            localWorldFavoriteGroups: isWorld
                ? state.localWorldFavoriteGroups
                : EMPTY_ARRAY,
            localAvatarFavoriteGroups: isAvatar
                ? state.localAvatarFavoriteGroups
                : EMPTY_ARRAY,
            localFriendFavoriteGroups: isFriend
                ? state.localFriendFavoriteGroups
                : EMPTY_ARRAY,
            localWorldDetailsById: isWorld
                ? state.localWorldDetailsById
                : EMPTY_OBJECT,
            favoriteWorldIds: isWorld ? state.favoriteWorldIds : EMPTY_ARRAY,
            favoriteAvatarIds: isAvatar ? state.favoriteAvatarIds : EMPTY_ARRAY
        };
    };
}
