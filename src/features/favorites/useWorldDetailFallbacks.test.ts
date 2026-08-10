import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { WorldProfileRecord } from '@/domain/entities/profileEntities';
import worldProfileRepository from '@/repositories/worldProfileRepository';

import {
    filterWorldDetailFallbacksById,
    getWorldDetailFallbackIds,
    loadWorldDetailFallbacksById
} from './useWorldDetailFallbacks';

vi.mock('@/repositories/worldProfileRepository', () => ({
    default: {
        getWorldProfile: vi.fn()
    }
}));

function cachedWorld(
    id: string,
    name: string,
    releaseStatus = 'private'
): WorldProfileRecord {
    return {
        id,
        authorId: 'usr_author',
        authorName: 'Cache Author',
        capacity: 0,
        created_at: '2026-06-01T00:00:00.000Z',
        createdAt: '2026-06-01T00:00:00.000Z',
        description: 'Cached description',
        favorites: 0,
        heat: 0,
        imageUrl: 'https://example.test/image.png',
        isLabs: false,
        name,
        occupants: 0,
        platforms: [],
        popularity: 0,
        publicationDate: null,
        recommendedCapacity: 0,
        releaseStatus,
        tags: [],
        thumbnailImageUrl: 'https://example.test/thumb.png',
        updated_at: '2026-06-02T00:00:00.000Z',
        updatedAt: '2026-06-02T00:00:00.000Z',
        version: 1,
        visits: 0
    };
}

function emptyWorld(id: string): WorldProfileRecord {
    return {
        ...cachedWorld(id, '', ''),
        authorId: '',
        authorName: '',
        createdAt: '',
        description: '',
        imageUrl: '',
        thumbnailImageUrl: '',
        updatedAt: '',
        version: 0
    };
}

describe('world detail fallback helpers', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('only asks the DB for remote favorite worlds with no displayable detail source', async () => {
        const fallbackIds = getWorldDetailFallbackIds({
            worldIds: [
                'wrld_remote',
                'wrld_fact',
                'wrld_local',
                'wrld_missing'
            ],
            kind: 'world',
            localWorldDetailsById: {
                wrld_local: { name: 'Local Baseline World' }
            },
            remoteEntityDetailsData: {
                wrld_remote: { name: 'Remote World' }
            },
            remoteEntityDetailsStatus: 'ready'
        });

        vi.mocked(worldProfileRepository.getWorldProfile).mockResolvedValue(
            cachedWorld('wrld_missing', 'DB Missing World')
        );

        const fallbacks = await loadWorldDetailFallbacksById(fallbackIds);

        expect(fallbackIds).toEqual(['wrld_fact', 'wrld_missing']);
        expect(worldProfileRepository.getWorldProfile).toHaveBeenCalledTimes(2);
        expect(worldProfileRepository.getWorldProfile).toHaveBeenCalledWith({
            worldId: 'wrld_missing'
        });
        expect(fallbacks).toMatchObject({
            wrld_missing: {
                name: 'DB Missing World',
                releaseStatus: 'private'
            }
        });
    });

    it('requests missing local favorite worlds through the same domain getter', () => {
        expect(
            getWorldDetailFallbackIds({
                worldIds: ['wrld_remote', 'wrld_local'],
                kind: 'world',
                localWorldDetailsById: {
                    wrld_remote: { name: 'Already Loaded' }
                },
                remoteEntityDetailsStatus: 'ready'
            })
        ).toEqual(['wrld_local']);
    });

    it('ignores empty cache shells returned from cache_world', async () => {
        vi.mocked(worldProfileRepository.getWorldProfile).mockImplementation(
            async ({ worldId }) => {
                if (worldId === 'wrld_cached') {
                    return cachedWorld('wrld_cached', 'Cached World', 'public');
                }
                if (worldId === 'wrld_shell') {
                    return emptyWorld('wrld_shell');
                }
                throw new Error('World not found');
            }
        );

        const fallbacks = await loadWorldDetailFallbacksById([
            'wrld_cached',
            'wrld_shell',
            'wrld_missing'
        ]);

        expect(fallbacks).toMatchObject({
            wrld_cached: {
                name: 'Cached World',
                releaseStatus: 'public'
            }
        });
        expect(fallbacks).not.toHaveProperty('wrld_shell');
        expect(fallbacks).not.toHaveProperty('wrld_missing');
    });

    it('filters stale fallback rows when the current favorite ids change', () => {
        const fallbacks = filterWorldDetailFallbacksById(
            {
                wrld_old: cachedWorld('wrld_old', 'Old World'),
                wrld_new: cachedWorld('wrld_new', 'New World')
            },
            ['wrld_new']
        );

        expect(fallbacks).toMatchObject({
            wrld_new: {
                name: 'New World'
            }
        });
        expect(fallbacks).not.toHaveProperty('wrld_old');
    });

    it('does not search cache_world before remote world details are ready', () => {
        expect(
            getWorldDetailFallbackIds({
                worldIds: ['wrld_pending'],
                kind: 'world',
                remoteEntityDetailsStatus: 'running'
            })
        ).toEqual([]);
    });
});
