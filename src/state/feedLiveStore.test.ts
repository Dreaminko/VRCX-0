import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { FeedLiveEntry, FeedLivePatch } from '@/domain/feed/feedLiveTypes';

import { feedEntryCorrectionId, useFeedLiveStore } from './feedLiveStore';
import { usePreferencesStore } from './preferencesStore';

const goldenFeedEntryCorrectionIds = [
    {
        input: {
            id: 'feed-entry-1',
            type: 'GPS',
            rowId: '10',
            sourceRank: '2'
        },
        expected: 'id:feed-entry-1'
    },
    {
        input: { type: 'GPS', rowId: '10', sourceRank: '2' },
        expected: 'row:GPS:2:10'
    },
    {
        input: { type: 'Online', row_id: '11', source_rank: '3' },
        expected: 'row:Online:3:11'
    },
    {
        input: {
            type: 'invite',
            created_at: '2026-06-21T00:00:00.000Z',
            userId: 'usr_sender',
            details: { location: 'wrld_world:123' },
            message: 'Join me'
        },
        expected:
            'invite:2026-06-21T00:00:00.000Z:usr_sender:wrld_world:123:Join me'
    }
];

function upsert(sequence: number, id: string): FeedLiveEntry {
    return { sequence, entry: { id } };
}

function patch(sequence: number, id: string): FeedLivePatch {
    return {
        sequence,
        id: `id:${id}`,
        fields: { displayName: `Name ${sequence}` }
    };
}

describe('feedEntryCorrectionId', () => {
    it('matches the Rust correction id golden vectors', () => {
        for (const vector of goldenFeedEntryCorrectionIds) {
            expect(feedEntryCorrectionId(vector.input)).toBe(vector.expected);
        }
    });
});

describe('feedLiveStore', () => {
    beforeEach(() => {
        useFeedLiveStore.getState().resetFeedLive();
        usePreferencesStore.setState((state) => ({
            ...state,
            feedPersistenceDisabled: false,
            tableLimits: { ...state.tableLimits, maxTableSize: 500 }
        }));
    });

    it('preserves Rust sequences and attaches the projection owner', () => {
        useFeedLiveStore
            .getState()
            .pushEntries([upsert(7, 'a'), upsert(9, 'b')], {
                ownerUserId: 'usr_owner'
            });

        const state = useFeedLiveStore.getState();
        expect(state.entries.map((entry) => entry.sequence)).toEqual([7, 9]);
        expect(state.entries.map((entry) => entry.entry.id)).toEqual([
            'a',
            'b'
        ]);
        expect(state.entries[1].ownerUserId).toBe('usr_owner');
        expect(state.entries[1].entry.ownerUserId).toBe('usr_owner');
        expect(state.version).toBe(9);
    });

    it('ignores invalid and already-observed sequences', () => {
        useFeedLiveStore.getState().pushEntries([upsert(5, 'a')]);
        useFeedLiveStore
            .getState()
            .pushEntries([
                upsert(4, 'old'),
                { sequence: 0, entry: { id: 'invalid' } },
                upsert(6, 'b'),
                null,
                undefined
            ]);

        const state = useFeedLiveStore.getState();
        expect(state.entries.map((entry) => entry.entry.id)).toEqual([
            'a',
            'b'
        ]);
        expect(state.version).toBe(6);
    });

    it('keeps the 100-entry frontend delta buffer while persistence is enabled', () => {
        useFeedLiveStore
            .getState()
            .pushEntries(
                Array.from({ length: 120 }, (_, index) =>
                    upsert(index + 1, `entry-${index}`)
                )
            );

        const state = useFeedLiveStore.getState();
        expect(state.entries).toHaveLength(100);
        expect(state.entries[0].sequence).toBe(21);
        expect(state.entries[99].sequence).toBe(120);
        expect(state.version).toBe(120);
    });

    it('uses the configured row limit while persistence is disabled', () => {
        usePreferencesStore.setState({ feedPersistenceDisabled: true });
        useFeedLiveStore
            .getState()
            .pushEntries(
                Array.from({ length: 520 }, (_, index) =>
                    upsert(index + 1, `entry-${index}`)
                )
            );

        const state = useFeedLiveStore.getState();
        expect(state.entries).toHaveLength(500);
        expect(state.entries[0].sequence).toBe(21);
        expect(state.entries[499].sequence).toBe(520);
    });

    it('applies sequenced corrections without changing the upsert order', () => {
        useFeedLiveStore
            .getState()
            .pushEntries([upsert(10, 'a'), upsert(11, 'b')]);
        useFeedLiveStore.getState().pushPatches([patch(12, 'a')]);

        const state = useFeedLiveStore.getState();
        expect(state.version).toBe(12);
        expect(state.entries.map((entry) => entry.entry.id)).toEqual([
            'a',
            'b'
        ]);
        expect(state.entries[0].sequence).toBe(10);
        expect(state.entries[0].entry.displayName).toBe('Name 12');
        expect(state.patches).toEqual([patch(12, 'a')]);
    });

    it('trims upserts and corrections without changing the watermark', () => {
        usePreferencesStore.setState({ feedPersistenceDisabled: true });
        useFeedLiveStore
            .getState()
            .pushEntries(
                Array.from({ length: 120 }, (_, index) =>
                    upsert(index + 1, `entry-${index}`)
                )
            );
        useFeedLiveStore
            .getState()
            .pushPatches(
                Array.from({ length: 120 }, (_, index) =>
                    patch(121 + index, `entry-${index}`)
                )
            );
        usePreferencesStore.setState((state) => ({
            tableLimits: { ...state.tableLimits, maxTableSize: 100 }
        }));

        useFeedLiveStore.getState().trimEntries();

        const state = useFeedLiveStore.getState();
        expect(state.entries).toHaveLength(100);
        expect(state.patches).toHaveLength(100);
        expect(state.entries[0].sequence).toBe(21);
        expect(state.patches[0].sequence).toBe(141);
        expect(state.version).toBe(240);
    });

    it('notifies subscribers once per batch', () => {
        const listener = vi.fn();
        const unsubscribe = useFeedLiveStore.subscribe(listener);
        useFeedLiveStore
            .getState()
            .pushEntries([upsert(1, 'a'), upsert(2, 'b'), upsert(3, 'c')]);
        unsubscribe();
        expect(listener).toHaveBeenCalledTimes(1);
    });
});
