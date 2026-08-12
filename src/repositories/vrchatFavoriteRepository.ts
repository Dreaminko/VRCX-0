import { commands, type VrchatFavoriteType } from '@/platform/tauri/bindings';

import { collectPages } from './pagination';
import { unwrapVrchatResponse } from './vrchatRequest';

const FAVORITE_GROUPS_PAGE_SIZE = 50;
const FAVORITE_DETAIL_PAGE_SIZE = 300;

type VrchatApiResult = {
    status: number;
    data: unknown;
};

interface FavoritePagingInput {
    n?: number;
    offset?: number;
}

interface FavoriteWorldsInput extends FavoritePagingInput {
    ownerId?: string;
    userId?: string;
    tag?: string;
}

interface FavoriteGroupsInput extends FavoritePagingInput {
    ownerId?: string;
}

interface FavoriteMutationInput {
    type?: unknown;
    favoriteId?: unknown;
    tags?: unknown;
}

interface DeleteFavoriteInput {
    objectId?: unknown;
}

interface FavoriteGroupMutationInput {
    type?: unknown;
    group?: unknown;
    displayName?: unknown;
    visibility?: unknown;
}

function requireVrchatFavoriteType(value: unknown): VrchatFavoriteType {
    if (
        value === 'friend' ||
        value === 'world' ||
        value === 'vrcPlusWorld' ||
        value === 'avatar'
    ) {
        return value;
    }
    throw new Error(
        'VrchatFavoriteRepository.addFavorite requires a valid favorite type.'
    );
}

function unwrapVrchatFavoriteResponse<TJson = unknown>(
    response: VrchatApiResult,
    path: string,
    fallbackMessage: string
) {
    return unwrapVrchatResponse<TJson>(response, path, { fallbackMessage });
}

async function addFavorite({
    type,
    favoriteId,
    tags
}: FavoriteMutationInput = {}) {
    const response = await commands.appVrchatFavoriteAdd({
        type: requireVrchatFavoriteType(type),
        favoriteId:
            typeof favoriteId === 'string'
                ? favoriteId
                : String(favoriteId ?? ''),
        tags: typeof tags === 'string' ? tags : String(tags ?? '')
    });
    return unwrapVrchatFavoriteResponse(
        response,
        'favorites',
        'VRChat favorite request failed'
    );
}

async function deleteFavorite({ objectId }: DeleteFavoriteInput = {}) {
    const normalizedObjectId =
        typeof objectId === 'string'
            ? objectId.trim()
            : String(objectId ?? '').trim();
    if (!normalizedObjectId) {
        throw new Error(
            'VrchatFavoriteRepository.deleteFavorite requires an object id.'
        );
    }

    const response = await commands.appVrchatFavoriteDelete({
        objectId: normalizedObjectId
    });
    return unwrapVrchatFavoriteResponse(
        response,
        `favorites/${encodeURIComponent(normalizedObjectId)}`,
        'VRChat favorite request failed'
    );
}

async function getFavoriteWorlds({
    n = FAVORITE_DETAIL_PAGE_SIZE,
    offset = 0,
    ownerId = '',
    userId = '',
    tag = ''
}: FavoriteWorldsInput = {}) {
    const response = await commands.appVrchatFavoriteWorldsGet({
        n,
        offset,
        ownerId,
        userId,
        tag
    });
    return unwrapVrchatFavoriteResponse(
        response,
        'worlds/favorites',
        'VRChat favorite request failed'
    );
}

async function getAllFavoriteWorlds({
    ownerId = '',
    userId = '',
    tag = ''
}: FavoriteWorldsInput = {}) {
    return collectPages(
        async ({ n, offset }) => {
            const response = await getFavoriteWorlds({
                n,
                offset,
                ownerId,
                userId,
                tag
            });
            return Array.isArray(response.json) ? response.json : [];
        },
        { pageSize: FAVORITE_DETAIL_PAGE_SIZE }
    );
}

async function getFavoriteGroups({
    n = FAVORITE_GROUPS_PAGE_SIZE,
    offset = 0,
    ownerId = ''
}: FavoriteGroupsInput = {}) {
    const response = await commands.appVrchatFavoriteGroupsGet({
        n,
        offset,
        ownerId
    });
    return unwrapVrchatFavoriteResponse(
        response,
        'favorite/groups',
        'VRChat favorite request failed'
    );
}

async function getAllFavoriteGroups({
    ownerId = ''
}: { ownerId?: string } = {}) {
    return collectPages(
        async ({ n, offset }) => {
            const response = await getFavoriteGroups({ n, offset, ownerId });
            return Array.isArray(response.json) ? response.json : [];
        },
        { pageSize: FAVORITE_GROUPS_PAGE_SIZE }
    );
}

async function saveFavoriteGroup({
    type,
    group,
    displayName,
    visibility
}: FavoriteGroupMutationInput = {}) {
    const normalizedType =
        typeof type === 'string' ? type.trim() : String(type ?? '').trim();
    const normalizedGroup =
        typeof group === 'string' ? group.trim() : String(group ?? '').trim();

    if (!normalizedType || !normalizedGroup) {
        throw new Error(
            'VrchatFavoriteRepository.saveFavoriteGroup requires type and group.'
        );
    }

    const response = await commands.appVrchatFavoriteGroupSave({
        type: normalizedType,
        group: normalizedGroup,
        displayName: typeof displayName === 'string' ? displayName : null,
        visibility: typeof visibility === 'string' ? visibility : null
    });
    return unwrapVrchatFavoriteResponse(
        response,
        `favorite/group/${encodeURIComponent(normalizedType)}/${encodeURIComponent(normalizedGroup)}`,
        'VRChat favorite request failed'
    );
}

async function clearFavoriteGroup({
    type,
    group
}: FavoriteGroupMutationInput = {}) {
    const normalizedType =
        typeof type === 'string' ? type.trim() : String(type ?? '').trim();
    const normalizedGroup =
        typeof group === 'string' ? group.trim() : String(group ?? '').trim();

    if (!normalizedType || !normalizedGroup) {
        throw new Error(
            'VrchatFavoriteRepository.clearFavoriteGroup requires type and group.'
        );
    }

    const response = await commands.appVrchatFavoriteGroupClear({
        type: normalizedType,
        group: normalizedGroup
    });
    return unwrapVrchatFavoriteResponse(
        response,
        `favorite/group/${encodeURIComponent(normalizedType)}/${encodeURIComponent(normalizedGroup)}`,
        'VRChat favorite request failed'
    );
}

const vrchatFavoriteRepository = Object.freeze({
    addFavorite,
    deleteFavorite,
    getAllFavoriteWorlds,
    getAllFavoriteGroups,
    saveFavoriteGroup,
    clearFavoriteGroup
});

export {
    addFavorite,
    deleteFavorite,
    getAllFavoriteWorlds,
    getAllFavoriteGroups,
    saveFavoriteGroup,
    clearFavoriteGroup
};
export default vrchatFavoriteRepository;
