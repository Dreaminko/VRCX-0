import { commands } from '@/platform/tauri/bindings';

import {
    type QueryParams,
    type VrchatRequestResponse,
    unwrapVrchatResponse
} from './vrchatRequest';

export type SearchEntityJson = Record<string, unknown> & {
    id: string;
};

export type SearchWorldJson = SearchEntityJson & {
    name?: string;
};

export type SearchUserJson = SearchEntityJson;
export type SearchGroupJson = SearchEntityJson & {
    bannerId: string | null;
    bannerUrl?: string;
    createdAt?: string;
    description?: string;
    discriminator?: string;
    galleries?: unknown[];
    iconId?: string;
    iconUrl?: string;
    isSearchable?: boolean;
    memberCount?: number;
    membershipStatus?: string;
    name?: string;
    ownerId?: string;
    rules?: string;
    shortCode?: string;
    tags?: unknown[];
};

export type SearchInstanceJson = Record<string, unknown> & {
    location?: unknown;
    shortName?: unknown;
    world?: unknown;
    worldName?: unknown;
};

function normalizeParams(params: QueryParams = {}): QueryParams {
    if (!params || typeof params !== 'object') {
        return {};
    }
    return { ...params };
}

type VrchatApiResult = {
    status: number;
    data: unknown;
};

function unwrapVrchatSearchResponse<TJson = unknown>(
    response: VrchatApiResult,
    path: string,
    params: QueryParams,
    extra: Record<string, unknown> = {},
    fallbackMessage: string = 'VRChat request failed'
): VrchatRequestResponse<TJson> {
    return {
        ...unwrapVrchatResponse<TJson>(response, path, { fallbackMessage }),
        params,
        ...extra
    };
}

async function getWorlds(params: QueryParams = {}, option?: unknown) {
    const normalizedParams = normalizeParams(params);
    const normalizedOption =
        typeof option === 'undefined' || option === null ? '' : String(option);
    const response = await commands.appVrchatSearchWorldsGet({
        params: normalizedParams,
        option: normalizedOption
    });
    const path = normalizedOption
        ? `worlds/${encodeURIComponent(normalizedOption)}`
        : 'worlds';
    return unwrapVrchatSearchResponse<SearchWorldJson[]>(
        response,
        path,
        normalizedParams,
        {
            option
        }
    );
}

async function getWorldById(worldId: unknown) {
    const normalizedWorldId = String(worldId || '').trim();
    const response = await commands.appVrchatSearchWorldsGet({
        params: {},
        option: normalizedWorldId
    });
    return unwrapVrchatSearchResponse<SearchWorldJson>(
        response,
        `worlds/${encodeURIComponent(normalizedWorldId)}`,
        {},
        { option: normalizedWorldId }
    );
}

async function getUsers(params: QueryParams = {}) {
    const normalizedParams = normalizeParams(params);
    const response = await commands.appVrchatSearchUsersGet({
        params: normalizedParams
    });
    return unwrapVrchatSearchResponse<SearchUserJson[]>(
        response,
        'users',
        normalizedParams
    );
}

async function getGroups(params: QueryParams = {}) {
    const normalizedParams = normalizeParams(params);
    const response = await commands.appVrchatSearchGroupsGet({
        params: normalizedParams
    });
    return unwrapVrchatSearchResponse<SearchGroupJson[]>(
        response,
        'groups',
        normalizedParams
    );
}

async function getGroupsStrictSearch(params: QueryParams = {}) {
    const normalizedParams = normalizeParams(params);
    const response = await commands.appVrchatSearchGroupsStrictGet({
        params: normalizedParams
    });
    return unwrapVrchatSearchResponse<SearchGroupJson[]>(
        response,
        'groups/strictsearch',
        normalizedParams
    );
}

async function getInstanceFromShortName(shortName: unknown) {
    const normalizedShortName = String(shortName || '').trim();
    const response = await commands.appVrchatSearchInstanceShortNameGet({
        shortName: normalizedShortName
    });
    return unwrapVrchatSearchResponse<SearchInstanceJson>(
        response,
        `instances/s/${encodeURIComponent(normalizedShortName)}`,
        {}
    );
}

const vrchatSearchRepository = Object.freeze({
    getWorlds,
    getWorldById,
    getUsers,
    getGroups,
    getGroupsStrictSearch,
    getInstanceFromShortName
});

export {
    getWorlds,
    getWorldById,
    getUsers,
    getGroups,
    getGroupsStrictSearch,
    getInstanceFromShortName
};
export default vrchatSearchRepository;
