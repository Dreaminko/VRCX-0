import type { FeedCursor } from '@/domain/feed/feedReadModelTypes';
import {
    commands,
    type FeedFilter,
    type FeedLatestQueryInput,
    type FeedQueryMode,
    type FeedRowOutput,
    type FeedRowsQueryInput,
    type FeedSearchQueryInput
} from '@/platform/tauri/bindings';
import {
    DEFAULT_MAX_TABLE_SIZE,
    DEFAULT_SEARCH_LIMIT
} from '@/shared/constants/settings';
import { normalizeString } from '@/shared/utils/string';

import { normalizeUserTablePrefix } from './userSessionRepository';

export type { FeedCursor } from '@/domain/feed/feedReadModelTypes';

interface FeedRowsQueryOptions {
    userId: unknown;
    mode: FeedQueryMode;
    search?: string;
    filters?: FeedFilter[];
    vipList?: string[];
    scopedUserIds?: string[];
    excludedUserIds?: string[];
    maxEntries?: number;
    dateFrom?: string;
    dateTo?: string;
    cursor?: FeedCursor | null;
}

interface FeedLatestQueryOptions {
    userId: unknown;
    filters?: FeedFilter[];
    favoriteUserIds?: string[];
    scopedUserIds?: string[];
    excludedUserIds?: string[];
    favoritesOnly?: boolean;
    maxRows?: number;
}

function normalizeStringList(value: unknown): string[] {
    return Array.isArray(value)
        ? value.map(normalizeString).filter(Boolean)
        : [];
}

const FEED_FILTER_SET: ReadonlySet<string> = new Set<FeedFilter>([
    'GPS',
    'Status',
    'Bio',
    'Avatar',
    'Online',
    'Offline'
]);

function normalizeFeedFilters(value: unknown): FeedFilter[] {
    return normalizeStringList(value).filter((filter): filter is FeedFilter =>
        FEED_FILTER_SET.has(filter)
    );
}

function getUserPrefix(userId: unknown) {
    return normalizeUserTablePrefix(userId);
}

const ensuredFeedTablePrefixes = new Map<string, Promise<void>>();

function ensureFeedTablesForUser(userId: unknown): Promise<void> {
    const userPrefix = getUserPrefix(userId);
    const existing = ensuredFeedTablePrefixes.get(userPrefix);
    if (existing) {
        return existing;
    }

    const promise = commands
        .appUserTablesEnsure(normalizeString(userId))
        .then((): void => undefined)
        .catch((error: unknown) => {
            if (ensuredFeedTablePrefixes.get(userPrefix) === promise) {
                ensuredFeedTablePrefixes.delete(userPrefix);
            }
            throw error;
        });
    ensuredFeedTablePrefixes.set(userPrefix, promise);
    return promise;
}

function markFeedTablesEnsured(userPrefix: unknown) {
    if (!userPrefix) {
        return;
    }
    ensuredFeedTablePrefixes.set(String(userPrefix), Promise.resolve());
}

async function queryFeedRows({
    userId,
    mode,
    search = '',
    filters = [],
    vipList = [],
    scopedUserIds = [],
    excludedUserIds = [],
    maxEntries = DEFAULT_MAX_TABLE_SIZE,
    dateFrom = '',
    dateTo = '',
    cursor = null
}: FeedRowsQueryOptions): Promise<FeedRowOutput[]> {
    await ensureFeedTablesForUser(userId);
    const query = {
        userId: normalizeString(userId),
        mode,
        search,
        filters: normalizeFeedFilters(filters),
        vipList: normalizeStringList(vipList),
        scopedUserIds: normalizeStringList(scopedUserIds),
        excludedUserIds: normalizeStringList(excludedUserIds),
        maxEntries,
        dateFrom,
        dateTo,
        cursor
    } satisfies FeedRowsQueryInput;
    return commands.appFeedRowsQuery(query);
}

const feed = {
    markFeedTablesEnsured,

    async searchFeedDatabase(
        search: string,
        filters: FeedFilter[],
        vipList: string[],
        maxEntries: number = DEFAULT_SEARCH_LIMIT,
        dateFrom: string = '',
        dateTo: string = '',
        userId: unknown = '',
        excludedUserIds: string[] = [],
        scopedUserIds: string[] = [],
        favoritesOnly: boolean = false
    ) {
        await ensureFeedTablesForUser(userId);
        const query = {
            userId: normalizeString(userId),
            search,
            filters: normalizeFeedFilters(filters),
            favoriteUserIds: normalizeStringList(vipList),
            scopedUserIds: normalizeStringList(scopedUserIds),
            excludedUserIds: normalizeStringList(excludedUserIds),
            favoritesOnly,
            dateFrom,
            dateTo,
            maxRows: maxEntries
        } satisfies FeedSearchQueryInput;
        return commands.appFeedSearchQuery(query);
    },

    async queryFeedLatest({
        userId,
        filters = [],
        favoriteUserIds = [],
        scopedUserIds = [],
        favoritesOnly = false,
        excludedUserIds = [],
        maxRows = DEFAULT_MAX_TABLE_SIZE
    }: FeedLatestQueryOptions) {
        await ensureFeedTablesForUser(userId);
        const query = {
            userId: normalizeString(userId),
            filters: normalizeFeedFilters(filters),
            favoriteUserIds: normalizeStringList(favoriteUserIds),
            scopedUserIds: normalizeStringList(scopedUserIds),
            favoritesOnly,
            excludedUserIds: normalizeStringList(excludedUserIds),
            maxRows
        } satisfies FeedLatestQueryInput;
        return commands.appFeedLatestQuery(query);
    },

    async lookupFeedDatabase(
        userId: unknown,
        filters: FeedFilter[],
        vipList: string[],
        maxEntries: number = DEFAULT_MAX_TABLE_SIZE,
        cursor: FeedCursor | null = null,
        excludedUserIds: string[] = [],
        scopedUserIds: string[] = []
    ) {
        return queryFeedRows({
            userId,
            mode: 'lookup',
            filters,
            vipList,
            scopedUserIds,
            excludedUserIds,
            maxEntries,
            cursor
        });
    },

    async getFeedByInstanceId(
        userId: unknown,
        instanceId: string,
        filters: FeedFilter[],
        vipList: string[],
        maxEntries: number = DEFAULT_SEARCH_LIMIT
    ) {
        return queryFeedRows({
            userId,
            mode: 'instance',
            search: instanceId,
            filters,
            vipList,
            maxEntries
        });
    }
};

export { feed };
export default feed;
