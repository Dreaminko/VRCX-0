import { describe, expect, it, vi } from 'vitest';

import { collectPages } from './pagination';

describe('collectPages', () => {
    it('stops after the first short page', async () => {
        const fetchPage = vi
            .fn()
            .mockResolvedValueOnce(['first', 'second'])
            .mockResolvedValueOnce(['third']);

        await expect(
            collectPages(fetchPage, { pageSize: 2, maxPages: 10 })
        ).resolves.toEqual(['first', 'second', 'third']);
        expect(fetchPage).toHaveBeenCalledTimes(2);
        expect(fetchPage).toHaveBeenNthCalledWith(1, { n: 2, offset: 0 });
        expect(fetchPage).toHaveBeenNthCalledWith(2, { n: 2, offset: 2 });
    });

    it('enforces the configured page limit for full pages', async () => {
        const fetchPage = vi.fn().mockResolvedValue(['row']);

        await expect(
            collectPages(fetchPage, { pageSize: 1, maxPages: 3 })
        ).resolves.toEqual(['row', 'row', 'row']);
        expect(fetchPage).toHaveBeenCalledTimes(3);
        expect(fetchPage).toHaveBeenLastCalledWith({ n: 1, offset: 2 });
    });
});
