// @vitest-environment jsdom

import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    fetchWorldProfile: vi.fn(),
    persistWorldDetails: vi.fn()
}));

vi.mock('@/repositories/worldProfileRepository', () => ({
    default: {
        fetchWorldProfile: mocks.fetchWorldProfile
    }
}));

vi.mock('@/services/favoriteWorldCacheService', () => ({
    persistWorldDetails: mocks.persistWorldDetails
}));

import { createRequestError } from '@/repositories/vrchatRequest';

import {
    clearWorldAvailabilityMemo,
    useWorldAvailabilityProbe
} from './useWorldAvailabilityProbe';

describe('useWorldAvailabilityProbe', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        clearWorldAvailabilityMemo();
    });

    it('marks a world private when the direct fetch succeeds', async () => {
        mocks.fetchWorldProfile.mockResolvedValue({
            id: 'wrld_private',
            releaseStatus: 'private'
        });

        const { result } = renderHook(() =>
            useWorldAvailabilityProbe({
                favoriteWorldIds: ['wrld_private'],
                kind: 'world',
                remoteEntityDetailsData: {},
                remoteEntityDetailsStatus: 'ready'
            })
        );

        await waitFor(() => {
            expect(result.current).toEqual({ wrld_private: 'private' });
        });

        expect(mocks.persistWorldDetails).toHaveBeenCalledWith(
            { id: 'wrld_private', releaseStatus: 'private' },
            'wrld_private'
        );
    });

    it('marks a world public when the direct fetch returns a public release status', async () => {
        mocks.fetchWorldProfile.mockResolvedValue({
            id: 'wrld_public',
            releaseStatus: 'public'
        });

        const { result } = renderHook(() =>
            useWorldAvailabilityProbe({
                favoriteWorldIds: ['wrld_public'],
                kind: 'world',
                remoteEntityDetailsData: {},
                remoteEntityDetailsStatus: 'ready'
            })
        );

        await waitFor(() => {
            expect(result.current).toEqual({ wrld_public: 'public' });
        });
    });

    it('marks a world deleted when the direct fetch 404s', async () => {
        mocks.fetchWorldProfile.mockRejectedValue(
            createRequestError('not found', 404, 'worlds/wrld_deleted')
        );

        const { result } = renderHook(() =>
            useWorldAvailabilityProbe({
                favoriteWorldIds: ['wrld_deleted'],
                kind: 'world',
                remoteEntityDetailsData: {},
                remoteEntityDetailsStatus: 'ready'
            })
        );

        await waitFor(() => {
            expect(result.current).toEqual({ wrld_deleted: 'deleted' });
        });
    });

    it('leaves availability unresolved on non-404 errors', async () => {
        mocks.fetchWorldProfile.mockRejectedValue(
            createRequestError('rate limited', 429, 'worlds/wrld_unknown')
        );

        const { result } = renderHook(() =>
            useWorldAvailabilityProbe({
                favoriteWorldIds: ['wrld_unknown'],
                kind: 'world',
                remoteEntityDetailsData: {},
                remoteEntityDetailsStatus: 'ready'
            })
        );

        await waitFor(() => {
            expect(mocks.fetchWorldProfile).toHaveBeenCalled();
        });

        expect(result.current).toEqual({});
    });

    it('reuses the session memo instead of probing again on remount', async () => {
        mocks.fetchWorldProfile.mockRejectedValue(
            createRequestError('not found', 404, 'worlds/wrld_cached')
        );

        const input = {
            favoriteWorldIds: ['wrld_cached'],
            kind: 'world',
            remoteEntityDetailsData: {},
            remoteEntityDetailsStatus: 'ready'
        } as const;

        const first = renderHook(() => useWorldAvailabilityProbe(input));
        await waitFor(() => {
            expect(first.result.current).toEqual({ wrld_cached: 'deleted' });
        });
        first.unmount();
        expect(mocks.fetchWorldProfile).toHaveBeenCalledTimes(1);

        const second = renderHook(() => useWorldAvailabilityProbe(input));
        await waitFor(() => {
            expect(second.result.current).toEqual({ wrld_cached: 'deleted' });
        });
        expect(mocks.fetchWorldProfile).toHaveBeenCalledTimes(1);
    });

    it('does not probe worlds already present in the batch response', async () => {
        renderHook(() =>
            useWorldAvailabilityProbe({
                favoriteWorldIds: ['wrld_present'],
                kind: 'world',
                remoteEntityDetailsData: {
                    wrld_present: { id: 'wrld_present', name: 'Present World' }
                },
                remoteEntityDetailsStatus: 'ready'
            })
        );

        await waitFor(() => {
            expect(mocks.fetchWorldProfile).not.toHaveBeenCalled();
        });
    });
});
