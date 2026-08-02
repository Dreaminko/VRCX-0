// @vitest-environment jsdom

import { cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const repositoryMock = vi.hoisted(() => ({
    getAvailableDates: vi.fn(),
    getInstanceActivityRows: vi.fn(),
    getWorldSummariesByIds: vi.fn()
}));

vi.mock('@/repositories/instanceActivityRepository', () => ({
    default: repositoryMock
}));

vi.mock('@/repositories/worldProfileRepository', () => ({
    default: {
        getWorldProfile: vi.fn()
    }
}));

import { useInstanceActivityData } from './useInstanceActivityData';

const firstRow = {
    id: 1,
    user_id: 'usr_a',
    display_name: 'A',
    location: 'wrld_a:1',
    created_at: '2026-07-01T10:00:00Z',
    time: 60_000
};

afterEach(() => {
    cleanup();
    vi.clearAllMocks();
});

describe('useInstanceActivityData', () => {
    it('hides rows and dates from the previous query as soon as the target changes', async () => {
        repositoryMock.getAvailableDates
            .mockResolvedValueOnce(['2026-07-01T10:00:00Z'])
            .mockImplementationOnce(() => new Promise(() => {}));
        repositoryMock.getInstanceActivityRows
            .mockResolvedValueOnce([firstRow])
            .mockImplementationOnce(() => new Promise(() => {}));
        repositoryMock.getWorldSummariesByIds.mockResolvedValue({
            wrld_a: { id: 'wrld_a', name: 'World A' }
        });

        const { result, rerender } = renderHook(
            (props: { currentUserId: string; selectedDate: string }) =>
                useInstanceActivityData({
                    currentEndpoint: 'https://api.vrchat.cloud',
                    currentUserId: props.currentUserId,
                    reloadToken: 0,
                    selectedDate: props.selectedDate
                }),
            {
                initialProps: {
                    currentUserId: 'usr_a',
                    selectedDate: '2026-07-01'
                }
            }
        );

        await waitFor(() => expect(result.current.dataStatus).toBe('ready'));
        expect(result.current.rawRows).toEqual([firstRow]);
        expect(result.current.availableDates).toEqual(['2026-07-01']);

        rerender({
            currentUserId: 'usr_b',
            selectedDate: '2026-07-02'
        });

        expect(result.current.dataStatus).toBe('running');
        expect(result.current.rawRows).toEqual([]);
        expect(result.current.availableDatesStatus).toBe('running');
        expect(result.current.availableDates).toEqual([]);
    });

    it('reports available-date failures independently from selected-day data', async () => {
        repositoryMock.getAvailableDates.mockRejectedValue(
            new Error('date query failed')
        );
        repositoryMock.getInstanceActivityRows.mockResolvedValue([firstRow]);
        repositoryMock.getWorldSummariesByIds.mockResolvedValue({
            wrld_a: { id: 'wrld_a', name: 'World A' }
        });

        const { result } = renderHook(() =>
            useInstanceActivityData({
                currentEndpoint: 'https://api.vrchat.cloud',
                currentUserId: 'usr_a',
                reloadToken: 0,
                selectedDate: '2026-07-01'
            })
        );

        await waitFor(() =>
            expect(result.current.availableDatesStatus).toBe('error')
        );
        await waitFor(() => expect(result.current.dataStatus).toBe('ready'));
        expect(result.current.availableDatesError).toBe('date query failed');
        expect(result.current.dataDetail).toBe('');
    });
});
