import type { FeedReadModelResult } from '@/domain/feed/feedReadModelTypes';
import type { FeedFilter, FeedRowOutput } from '@/platform/tauri/bindings';

import configRepository from './configRepository';
import feedPersistenceRepository from './feedPersistenceRepository';
import type { FeedCursor } from './feedPersistenceRepository';
import userSessionRepository from './userSessionRepository';

export const FEED_FILTER_TYPES: readonly FeedFilter[] = Object.freeze([
    'GPS',
    'Online',
    'Offline',
    'Status',
    'Avatar',
    'Bio'
]);

export type FeedFilterType = FeedFilter;
export type FeedEntry = Record<string, unknown>;
const FEED_FILTER_TYPE_SET: ReadonlySet<string> = new Set(FEED_FILTER_TYPES);

export interface FeedQueryOptions {
    userId: unknown;
    search?: unknown;
    filters?: unknown[];
    favoriteUserIds?: unknown[];
    scopedUserIds?: readonly unknown[];
    excludedFavoriteUserIds?: unknown[];
    dateFrom?: string;
    dateTo?: string;
    maxEntries?: number;
    cursor?: FeedCursor | null;
    favoritesOnly?: boolean;
}

export interface FeedLatestQueryOptions {
    userId: unknown;
    filters?: unknown[];
    favoriteUserIds?: unknown[];
    scopedUserIds?: readonly unknown[];
    excludedFavoriteUserIds?: unknown[];
    favoritesOnly?: boolean;
    maxRows?: number;
}

interface FeedReadyState {
    normalizedUserId: string;
    maxTableSize: number;
    searchLimit: number;
}

function normalizeUserId(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeUserIdList(value: readonly unknown[] = []): string[] {
    return Array.from(
        new Set(
            (Array.isArray(value) ? value : [])
                .map((entry) => normalizeUserId(entry))
                .filter(Boolean)
        )
    );
}

function normalizeFilterList(filters: unknown[] = []): FeedFilterType[] {
    if (!Array.isArray(filters)) {
        return [];
    }

    return filters.filter((filter, index, source): filter is FeedFilterType => {
        if (typeof filter !== 'string') {
            return false;
        }

        if (!FEED_FILTER_TYPE_SET.has(filter)) {
            return false;
        }

        return source.indexOf(filter) === index;
    });
}

class FeedRepository {
    #currentUserId: string = '';

    async #ensureReady(userId: unknown): Promise<FeedReadyState> {
        const normalizedUserId = normalizeUserId(userId);
        if (!normalizedUserId) {
            throw new Error('FeedRepository requires a current user id.');
        }

        const [maxTableSize, searchLimit] = await Promise.all([
            configRepository.getInt('maxTableSize_v2', 500),
            configRepository.getInt('searchLimit', 50000)
        ]);

        if (this.#currentUserId !== normalizedUserId) {
            await userSessionRepository.ensureUserTables(normalizedUserId);
            this.#currentUserId = normalizedUserId;
        }

        return {
            normalizedUserId,
            maxTableSize: Number(maxTableSize),
            searchLimit: Number(searchLimit)
        };
    }

    async queryFeed({
        userId,
        search = '',
        filters = [],
        favoriteUserIds = [],
        scopedUserIds = [],
        excludedFavoriteUserIds = [],
        dateFrom = '',
        dateTo = '',
        maxEntries,
        cursor = null,
        favoritesOnly = false
    }: FeedQueryOptions): Promise<FeedRowOutput[]> {
        const { normalizedUserId, maxTableSize, searchLimit } =
            await this.#ensureReady(userId);
        const normalizedFilters = normalizeFilterList(filters);
        const normalizedFavorites = normalizeUserIdList(favoriteUserIds);
        const normalizedScoped = normalizeUserIdList(scopedUserIds);
        const normalizedExcludedFavorites = normalizeUserIdList(
            excludedFavoriteUserIds
        );
        const normalizedSearch = String(search || '').trim();

        if (normalizedSearch || dateFrom || dateTo) {
            return feedPersistenceRepository.searchFeedDatabase(
                normalizedSearch,
                normalizedFilters,
                normalizedFavorites,
                maxEntries ?? searchLimit,
                dateFrom,
                dateTo,
                normalizedUserId,
                normalizedExcludedFavorites,
                normalizedScoped,
                favoritesOnly
            );
        }

        return feedPersistenceRepository.lookupFeedDatabase(
            normalizedUserId,
            normalizedFilters,
            normalizedFavorites,
            maxEntries ?? maxTableSize,
            cursor,
            normalizedExcludedFavorites,
            normalizedScoped
        );
    }

    async queryFeedPage(options: FeedQueryOptions): Promise<FeedRowOutput[]> {
        return this.queryFeed(options);
    }

    async queryFeedLatest({
        userId,
        filters = [],
        favoriteUserIds = [],
        scopedUserIds = [],
        excludedFavoriteUserIds = [],
        favoritesOnly = false,
        maxRows
    }: FeedLatestQueryOptions): Promise<FeedReadModelResult<FeedRowOutput>> {
        const { normalizedUserId, maxTableSize } =
            await this.#ensureReady(userId);
        const normalizedFilters = normalizeFilterList(filters);
        const normalizedFavorites = normalizeUserIdList(favoriteUserIds);
        const normalizedScoped = normalizeUserIdList(scopedUserIds);
        const normalizedExcludedFavorites = normalizeUserIdList(
            excludedFavoriteUserIds
        );

        return feedPersistenceRepository.queryFeedLatest({
            userId: normalizedUserId,
            filters: normalizedFilters,
            favoriteUserIds: normalizedFavorites,
            scopedUserIds: normalizedScoped,
            excludedUserIds: normalizedExcludedFavorites,
            favoritesOnly,
            maxRows: maxRows ?? maxTableSize
        });
    }
}

const feedRepository = new FeedRepository();

export { FeedRepository };
export default feedRepository;
