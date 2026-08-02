// @vitest-environment jsdom

import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import {
    EMPTY_FALLBACKS,
    getRemoteEntityCacheFallbackIds,
    loadRemoteEntityCacheFallbacksById,
    useRemoteEntityCacheFallbackLoader
} from './remoteEntityCacheFallbacks';

function deferred<T>() {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>((resolvePromise) => {
        resolve = resolvePromise;
    });
    return { promise, resolve };
}

describe('remoteEntityCacheFallbacks', () => {
    it('deduplicates ids and skips entities already available from any detail source', () => {
        expect(
            getRemoteEntityCacheFallbackIds({
                entityIds: [' wrld_a ', 'wrld_a', 'wrld_b', ''],
                detailSources: [
                    { wrld_a: { name: 'Cached World' } },
                    { wrld_b: {} }
                ],
                isReady: true
            })
        ).toEqual(['wrld_b']);
        expect(
            getRemoteEntityCacheFallbackIds({
                entityIds: ['wrld_a'],
                detailSources: [],
                isReady: false
            })
        ).toEqual([]);
    });

    it('keeps successful displayable results when other fetches fail', async () => {
        const fetchById = vi.fn(async (id: string) => {
            if (id === 'wrld_failed') {
                throw new Error('offline');
            }
            if (id === 'wrld_empty') {
                return { id };
            }
            return { id: 'wrld_canonical', name: 'Canonical World' };
        });

        await expect(
            loadRemoteEntityCacheFallbacksById(
                ['wrld_query', 'wrld_failed', 'wrld_empty'],
                fetchById
            )
        ).resolves.toEqual({
            wrld_canonical: {
                id: 'wrld_canonical',
                name: 'Canonical World'
            }
        });
    });

    it('returns the shared empty value when no fallback is usable', async () => {
        await expect(
            loadRemoteEntityCacheFallbacksById([], vi.fn())
        ).resolves.toBe(EMPTY_FALLBACKS);
        await expect(
            loadRemoteEntityCacheFallbacksById(['wrld_a'], async () => ({}))
        ).resolves.toBe(EMPTY_FALLBACKS);
    });

    it('ignores stale async results after the requested id set changes', async () => {
        const first = deferred<{ id: string; name: string }>();
        const second = deferred<{ id: string; name: string }>();
        const fetchById = vi.fn((id: string) =>
            id === 'wrld_first' ? first.promise : second.promise
        );
        const { result, rerender } = renderHook(
            ({ ids }) => useRemoteEntityCacheFallbackLoader(ids, fetchById),
            { initialProps: { ids: ['wrld_first'] } }
        );

        rerender({ ids: ['wrld_second'] });
        await act(async () => {
            second.resolve({ id: 'wrld_second', name: 'Second' });
        });
        await waitFor(() => {
            expect(result.current).toEqual({
                wrld_second: { id: 'wrld_second', name: 'Second' }
            });
        });

        await act(async () => {
            first.resolve({ id: 'wrld_first', name: 'First' });
        });
        expect(result.current).toEqual({
            wrld_second: { id: 'wrld_second', name: 'Second' }
        });
    });
});
