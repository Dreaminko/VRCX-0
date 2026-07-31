// @vitest-environment jsdom

import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    appFavoriteDetailsHydrate: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appFavoriteDetailsHydrate: mocks.appFavoriteDetailsHydrate
    }
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: <T,>(
        selector: (state: { auth: { currentUserEndpoint: string } }) => T
    ): T =>
        selector({
            auth: { currentUserEndpoint: 'https://api.vrchat.cloud' }
        })
}));

import {
    bumpFavoriteRemoteDetailsRefresh,
    useFavoriteRemoteDetails
} from './useFavoriteRemoteDetails';

describe('useFavoriteRemoteDetails', () => {
    afterEach(() => {
        cleanup();
    });

    beforeEach(() => {
        vi.clearAllMocks();
        mocks.appFavoriteDetailsHydrate.mockResolvedValue({
            detailsById: {},
            cachedCount: 0,
            fetchedAt: '2026-07-31T00:00:00.000Z'
        });
    });

    it('hydrates remote details through the backend command', async () => {
        mocks.appFavoriteDetailsHydrate.mockResolvedValue({
            detailsById: {
                wrld_1: {
                    id: 'wrld_1',
                    name: 'World One',
                    releaseStatus: 'public'
                }
            },
            cachedCount: 1,
            fetchedAt: '2026-07-31T00:00:00.000Z'
        });

        const { result } = renderHook(() =>
            useFavoriteRemoteDetails({
                type: 'world',
                favoriteIds: ['wrld_1', ' wrld_2 ']
            })
        );

        await waitFor(() => {
            expect(result.current.status).toBe('ready');
        });

        expect(mocks.appFavoriteDetailsHydrate).toHaveBeenCalledWith({
            kind: 'world',
            favoriteIds: ['wrld_1', 'wrld_2'],
            avatarTags: []
        });
        expect(result.current.data).toEqual({
            wrld_1: {
                id: 'wrld_1',
                name: 'World One',
                releaseStatus: 'public'
            }
        });
        expect(result.current.lastLoadedAt).toBe('2026-07-31T00:00:00.000Z');
    });

    it('passes normalized avatar tags for avatar hydration', async () => {
        const { result } = renderHook(() =>
            useFavoriteRemoteDetails({
                type: 'avatar',
                favoriteIds: ['avtr_1'],
                avatarTags: [' one ', 'one', 'two']
            })
        );

        await waitFor(() => {
            expect(result.current.status).toBe('ready');
        });

        expect(mocks.appFavoriteDetailsHydrate).toHaveBeenCalledWith({
            kind: 'avatar',
            favoriteIds: ['avtr_1'],
            avatarTags: ['one', 'two']
        });
    });

    it('stays ready without calling the backend when disabled or without ids', async () => {
        const { result } = renderHook(() =>
            useFavoriteRemoteDetails({
                type: 'world',
                favoriteIds: [],
                enabled: true
            })
        );

        await waitFor(() => {
            expect(result.current.status).toBe('ready');
        });
        expect(mocks.appFavoriteDetailsHydrate).not.toHaveBeenCalled();
    });

    it('does not refetch on rerender with unchanged parameters', async () => {
        const { rerender, result } = renderHook(
            ({ refreshToken }: { refreshToken: number }) =>
                useFavoriteRemoteDetails({
                    type: 'world',
                    favoriteIds: ['wrld_1'],
                    refreshToken
                }),
            { initialProps: { refreshToken: 0 } }
        );

        await waitFor(() => {
            expect(result.current.status).toBe('ready');
        });
        rerender({ refreshToken: 0 });
        await waitFor(() => {
            expect(result.current.status).toBe('ready');
        });

        expect(mocks.appFavoriteDetailsHydrate).toHaveBeenCalledTimes(1);
    });

    it('refetches when the refresh token changes', async () => {
        const { rerender, result } = renderHook(
            ({ refreshToken }: { refreshToken: number }) =>
                useFavoriteRemoteDetails({
                    type: 'world',
                    favoriteIds: ['wrld_1'],
                    refreshToken
                }),
            { initialProps: { refreshToken: 0 } }
        );

        await waitFor(() => {
            expect(result.current.status).toBe('ready');
        });
        rerender({ refreshToken: 1 });
        await waitFor(() => {
            expect(mocks.appFavoriteDetailsHydrate).toHaveBeenCalledTimes(2);
        });
    });

    it('refetches when the module-level refresh signal is bumped', async () => {
        const { result } = renderHook(() =>
            useFavoriteRemoteDetails({
                type: 'world',
                favoriteIds: ['wrld_1']
            })
        );

        await waitFor(() => {
            expect(result.current.status).toBe('ready');
        });
        act(() => {
            bumpFavoriteRemoteDetailsRefresh();
        });
        await waitFor(() => {
            expect(mocks.appFavoriteDetailsHydrate).toHaveBeenCalledTimes(2);
        });
    });

    it('surfaces backend failures as an error state', async () => {
        mocks.appFavoriteDetailsHydrate.mockRejectedValue(
            new Error('hydrate failed')
        );

        const { result } = renderHook(() =>
            useFavoriteRemoteDetails({
                type: 'avatar',
                favoriteIds: ['avtr_1']
            })
        );

        await waitFor(() => {
            expect(result.current.status).toBe('error');
        });
        expect(result.current.detail).toBe('hydrate failed');
        expect(result.current.data).toEqual({});
    });
});
