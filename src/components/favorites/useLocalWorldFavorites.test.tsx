// @vitest-environment jsdom

import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    loadLocalWorldFavoritesSnapshot: vi.fn()
}));

vi.mock('@/services/localWorldFavoritesService', () => ({
    loadLocalWorldFavoritesSnapshot: mocks.loadLocalWorldFavoritesSnapshot
}));

import { useFavoriteRevisionStore } from '@/state/favoriteRevisionStore';

import { useLocalWorldFavorites } from './useLocalWorldFavorites';

describe('useLocalWorldFavorites', () => {
    afterEach(() => {
        cleanup();
    });

    beforeEach(() => {
        vi.clearAllMocks();
        useFavoriteRevisionStore.getState().reset();
        mocks.loadLocalWorldFavoritesSnapshot.mockResolvedValue({
            favoritesByGroup: { First: ['wrld_1'] },
            groupNames: ['First']
        });
    });

    it('rereads the Rust-backed snapshot after a favorites revision', async () => {
        const { result } = renderHook(() => useLocalWorldFavorites());

        await waitFor(() => {
            expect(result.current.status).toBe('ready');
        });
        expect(result.current.groupNames).toEqual(['First']);

        mocks.loadLocalWorldFavoritesSnapshot.mockResolvedValue({
            favoritesByGroup: { Second: ['wrld_2'] },
            groupNames: ['Second']
        });
        act(() => {
            useFavoriteRevisionStore.getState().bumpRevision({
                kind: 'world',
                local: true,
                remote: false,
                requiresRefresh: true
            });
        });

        await waitFor(() => {
            expect(result.current.groupNames).toEqual(['Second']);
        });
        expect(mocks.loadLocalWorldFavoritesSnapshot).toHaveBeenCalledTimes(2);
    });

    it('does not read persistence while disabled', () => {
        const { result } = renderHook(() => useLocalWorldFavorites(false));

        expect(result.current.status).toBe('idle');
        expect(mocks.loadLocalWorldFavoritesSnapshot).not.toHaveBeenCalled();
    });

    it('reports read failures without rejecting a completed write flow', async () => {
        mocks.loadLocalWorldFavoritesSnapshot.mockRejectedValue(
            new Error('read failed')
        );
        const { result } = renderHook(() => useLocalWorldFavorites());

        await waitFor(() => {
            expect(result.current.status).toBe('error');
        });
        await expect(result.current.reload()).resolves.toBe(false);
    });
});
