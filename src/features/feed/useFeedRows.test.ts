// @vitest-environment jsdom

import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    queryFeedLatest: vi.fn(),
    queryFeed: vi.fn(),
    getFriendLogCurrent: vi.fn(),
    getAllUserStats: vi.fn(),
    runtime: { auth: { currentUserId: 'usr_self' } },
    session: { isFavoritesLoaded: true },
    favorites: {
        remoteFavoritesById: {} as Record<string, unknown>,
        localFriendFavorites: {} as Record<string, unknown>
    },
    preferences: {
        localFavoriteFriendsGroups: [] as string[],
        feedHiddenUsers: [] as string[],
        feedPersistenceDisabled: false,
        tableLimits: { maxTableSize: 100 }
    },
    friendLog: { revision: 0 }
}));

vi.mock('@/repositories/feedRepository', () => ({
    default: {
        queryFeedLatest: mocks.queryFeedLatest,
        queryFeed: mocks.queryFeed
    }
}));

vi.mock('@/repositories/friendLogRepository', () => ({
    default: { getFriendLogCurrent: mocks.getFriendLogCurrent }
}));

vi.mock('@/repositories/gameLogRepository', () => ({
    default: { getAllUserStats: mocks.getAllUserStats }
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: <T>(selector: (state: typeof mocks.runtime) => T): T =>
        selector(mocks.runtime)
}));

vi.mock('@/state/sessionStore', () => ({
    useSessionStore: <T>(selector: (state: typeof mocks.session) => T): T =>
        selector(mocks.session)
}));

vi.mock('@/state/favoriteStore', () => ({
    useFavoriteStore: <T>(selector: (state: typeof mocks.favorites) => T): T =>
        selector(mocks.favorites)
}));

vi.mock('@/state/preferencesStore', () => ({
    usePreferencesStore: Object.assign(
        <T>(selector: (state: typeof mocks.preferences) => T): T =>
            selector(mocks.preferences),
        { getState: () => mocks.preferences }
    )
}));

vi.mock('@/state/friendLogStore', () => ({
    useFriendLogStore: <T>(selector: (state: typeof mocks.friendLog) => T): T =>
        selector(mocks.friendLog)
}));

import { useFeedLiveStore } from '@/state/feedLiveStore';

import { createDeferred, flush, pushLiveEntry } from './feedLiveMergeTestUtils';
import type { FeedFilterType, FeedRow } from './feedTypes';
import { useFeedRows } from './useFeedRows';

type FeedRowsProps = {
    activeFilters: FeedFilterType[];
    dateFrom: string;
    dateTo: string;
    deferredSearchQuery: string;
    favoritesOnly: boolean;
    scopedUserIds: readonly string[];
    preferencesReady: boolean;
};

const BASE_PROPS: FeedRowsProps = {
    activeFilters: [],
    dateFrom: '',
    dateTo: '',
    deferredSearchQuery: '',
    favoritesOnly: false,
    scopedUserIds: [],
    preferencesReady: true
};

function renderFeedRows() {
    return renderHook((props: FeedRowsProps) => useFeedRows(props), {
        initialProps: BASE_PROPS
    });
}

describe('useFeedRows', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.clearAllMocks();
        mocks.preferences.feedPersistenceDisabled = false;
        mocks.friendLog.revision = 0;
        useFeedLiveStore.getState().resetFeedLive();
        mocks.getFriendLogCurrent.mockResolvedValue([]);
        mocks.getAllUserStats.mockResolvedValue([]);
        mocks.queryFeedLatest.mockResolvedValue({ rows: [], maxSequence: 0 });
        mocks.queryFeed.mockResolvedValue([]);
    });

    afterEach(() => {
        cleanup();
        vi.useRealTimers();
    });

    it('loads the latest snapshot and applies realtime rows without another IPC', async () => {
        mocks.queryFeedLatest.mockResolvedValue({
            rows: [{ userId: 'usr_base' }],
            maxSequence: 0
        });
        const { result } = renderFeedRows();
        await flush();

        pushLiveEntry('live');
        await flush();

        expect(result.current.rows.map((row) => row.userId)).toEqual([
            'usr_live',
            'usr_base'
        ]);
        expect(mocks.queryFeedLatest).toHaveBeenCalledTimes(1);
        expect(mocks.queryFeed).not.toHaveBeenCalled();
    });

    it('applies a correction event even when no upsert remains in the frontend buffer', async () => {
        mocks.queryFeedLatest.mockResolvedValue({
            rows: [
                {
                    rowId: 1,
                    sourceRank: 60,
                    type: 'GPS',
                    worldName: 'wrld_1'
                }
            ],
            maxSequence: 0
        });
        const { result } = renderFeedRows();
        await flush();

        act(() => {
            useFeedLiveStore.getState().pushPatches([
                {
                    sequence: 1,
                    id: 'row:GPS:60:1',
                    fields: { worldName: 'Resolved World' }
                }
            ]);
        });
        await flush();

        expect(result.current.rows[0].worldName).toBe('Resolved World');
        expect(mocks.queryFeedLatest).toHaveBeenCalledTimes(1);
    });

    it('normalizes snake-case fields on realtime entries', async () => {
        const { result } = renderFeedRows();
        await flush();

        act(() => {
            useFeedLiveStore.getState().pushEntries(
                [
                    {
                        sequence: 1,
                        entry: {
                            type: 'GPS',
                            user_id: 'usr_snake',
                            display_name: 'Snake Friend',
                            createdAt: '2026-05-15T00:00:00Z',
                            location: 'wrld_1:instance',
                            world_name: 'Snake World',
                            time: '1500'
                        }
                    }
                ],
                { ownerUserId: 'usr_self' }
            );
        });
        await flush();

        expect(result.current.rows[0]).toMatchObject({
            userId: 'usr_snake',
            displayName: 'Snake Friend',
            worldName: 'Snake World',
            time: 1500
        });
    });

    it('keeps search results static while realtime entries continue arriving', async () => {
        mocks.queryFeed.mockResolvedValue([{ userId: 'usr_search' }]);
        const { result } = renderHook(
            (props: FeedRowsProps) => useFeedRows(props),
            {
                initialProps: {
                    ...BASE_PROPS,
                    deferredSearchQuery: 'needle'
                }
            }
        );
        await flush();

        pushLiveEntry('ignored-by-search');
        await act(async () => {
            vi.advanceTimersByTime(250);
        });
        await flush();

        expect(result.current.rows).toEqual([{ userId: 'usr_search' }]);
        expect(mocks.queryFeed).toHaveBeenCalledWith(
            expect.objectContaining({ search: 'needle', maxEntries: 100 })
        );
        expect(mocks.queryFeedLatest).not.toHaveBeenCalled();
    });

    it('reloads the latest snapshot after search is cleared', async () => {
        mocks.queryFeed.mockResolvedValue([{ userId: 'usr_search' }]);
        mocks.queryFeedLatest.mockResolvedValue({
            rows: [{ userId: 'usr_latest' }],
            maxSequence: 4
        });
        const { result, rerender } = renderHook(
            (props: FeedRowsProps) => useFeedRows(props),
            {
                initialProps: {
                    ...BASE_PROPS,
                    deferredSearchQuery: 'needle'
                }
            }
        );
        await flush();

        rerender(BASE_PROPS);
        await flush();

        expect(result.current.rows).toEqual([{ userId: 'usr_latest' }]);
        expect(mocks.queryFeedLatest).toHaveBeenCalledTimes(1);
    });

    it('does not let a stale search response overwrite the resynced latest rows', async () => {
        const search = createDeferred<FeedRow[]>();
        mocks.queryFeed.mockReturnValue(search.promise);
        mocks.queryFeedLatest.mockResolvedValue({
            rows: [{ userId: 'usr_latest' }],
            maxSequence: 0
        });
        const { result, rerender } = renderHook(
            (props: FeedRowsProps) => useFeedRows(props),
            {
                initialProps: {
                    ...BASE_PROPS,
                    deferredSearchQuery: 'needle'
                }
            }
        );

        rerender(BASE_PROPS);
        await flush();
        await act(async () => {
            search.resolve([{ userId: 'usr_stale' }]);
        });
        await flush();

        expect(result.current.rows).toEqual([{ userId: 'usr_latest' }]);
    });

    it('uses the Rust latest snapshot when persistence is disabled', async () => {
        mocks.preferences.feedPersistenceDisabled = true;
        const { result } = renderFeedRows();
        await flush();

        expect(result.current.loadStatus).toBe('ready');
        expect(mocks.queryFeedLatest).toHaveBeenCalledTimes(1);
        expect(mocks.queryFeed).not.toHaveBeenCalled();
    });

    it('accepts restarted Rust sequences after the persistence mode changes', async () => {
        mocks.queryFeedLatest.mockResolvedValue({ rows: [], maxSequence: 9 });
        const { result, rerender } = renderFeedRows();
        await flush();

        useFeedLiveStore.getState().resetFeedLive();
        mocks.preferences.feedPersistenceDisabled = true;
        mocks.queryFeedLatest.mockResolvedValue({ rows: [], maxSequence: 0 });
        rerender(BASE_PROPS);
        await flush();

        pushLiveEntry('restarted');
        await flush();

        expect(result.current.rows.map((row) => row.userId)).toEqual([
            'usr_restarted'
        ]);
    });
});
