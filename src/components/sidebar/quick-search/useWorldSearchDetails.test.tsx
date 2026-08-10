// @vitest-environment jsdom

import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    searchWorlds: vi.fn()
}));

vi.mock('@/repositories/worldProfileRepository', () => ({
    default: {
        searchWorlds: mocks.searchWorlds
    }
}));

import { useWorldSearchDetails } from './useWorldSearchDetails';

describe('useWorldSearchDetails', () => {
    beforeEach(() => {
        mocks.searchWorlds.mockReset();
        mocks.searchWorlds.mockResolvedValue([
            {
                id: 'wrld_cached',
                name: 'Cached World',
                authorId: 'usr_author',
                authorName: 'Author',
                created_at: '',
                description: 'Description',
                imageUrl: 'https://example.test/world.png',
                releaseStatus: 'public',
                thumbnailImageUrl: 'https://example.test/world-thumb.png',
                updated_at: '',
                version: 1
            }
        ]);
    });

    it('requests only matching world rows after detail search starts', async () => {
        const { result, rerender } = renderHook(
            ({ normalizedQuery }) => useWorldSearchDetails(normalizedQuery),
            {
                initialProps: { normalizedQuery: 'w' }
            }
        );

        expect(mocks.searchWorlds).not.toHaveBeenCalled();
        expect(result.current).toEqual({});

        rerender({ normalizedQuery: 'wo' });

        await waitFor(() => {
            expect(result.current.wrld_cached).toMatchObject({
                id: 'wrld_cached',
                name: 'Cached World'
            });
        });
        expect(mocks.searchWorlds).toHaveBeenCalledWith('wo');

        rerender({ normalizedQuery: 'world' });
        await waitFor(() => {
            expect(mocks.searchWorlds).toHaveBeenLastCalledWith('world');
        });
    });
});
