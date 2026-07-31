import favoritePersistenceRepository, {
    type FavoriteCacheEntity
} from '@/repositories/favoritePersistenceRepository';
import { useFavoriteStore } from '@/state/favoriteStore';

type WorldCacheSource = Record<string, unknown>;

function isRecord(value: unknown): value is WorldCacheSource {
    return Boolean(value && typeof value === 'object');
}

function normalizeEntityId(value: unknown) {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeString(value: unknown) {
    return typeof value === 'string' ? value : String(value ?? '');
}

function normalizeReleaseStatus(world: unknown) {
    return normalizeEntityId(
        isRecord(world) ? world.releaseStatus : undefined
    ).toLowerCase();
}

function hasCompleteWorldSnapshot(world: unknown) {
    const source = isRecord(world) ? world : {};
    const name = normalizeString(source.name).trim();
    const imageUrl =
        normalizeString(source.thumbnailImageUrl).trim() ||
        normalizeString(source.imageUrl).trim();
    return Boolean(name && imageUrl);
}

function canUpsertWorldSnapshot(world: unknown) {
    return (
        normalizeReleaseStatus(world) === 'public' &&
        hasCompleteWorldSnapshot(world)
    );
}

function canInsertMissingWorldSnapshot(world: unknown) {
    return (
        normalizeReleaseStatus(world) === 'private' &&
        hasCompleteWorldSnapshot(world)
    );
}

function buildWorldCacheEntry(
    world: unknown,
    fallbackWorldId?: unknown
): FavoriteCacheEntity | null {
    if (!isRecord(world)) {
        return null;
    }

    const id =
        normalizeEntityId(world.id) || normalizeEntityId(fallbackWorldId);
    if (!id) {
        return null;
    }

    if (
        !canUpsertWorldSnapshot(world) &&
        !canInsertMissingWorldSnapshot(world)
    ) {
        return null;
    }

    return {
        id,
        authorId: normalizeEntityId(world.authorId),
        authorName: normalizeString(world.authorName),
        created_at: normalizeString(world.created_at ?? world.createdAt),
        description: normalizeString(world.description),
        imageUrl: normalizeString(world.imageUrl),
        name: normalizeString(world.name),
        releaseStatus: normalizeString(world.releaseStatus),
        thumbnailImageUrl: normalizeString(world.thumbnailImageUrl),
        updated_at: normalizeString(world.updated_at ?? world.updatedAt),
        version: Number(world.version) || 0
    };
}

export async function cacheWorldDetails(
    world: unknown,
    fallbackWorldId?: unknown
) {
    const entry = buildWorldCacheEntry(world, fallbackWorldId);
    if (!entry) {
        return false;
    }

    const canUpsert = canUpsertWorldSnapshot(world);
    if (!canUpsert) {
        const existing = await favoritePersistenceRepository.getCachedWorldById(
            entry.id
        );
        if (existing) {
            return false;
        }
    }

    await favoritePersistenceRepository.addWorldToCache(entry);
    return true;
}

function isFavoriteWorldId(id: string) {
    const state = useFavoriteStore.getState();
    return (
        state.favoriteWorldIds.includes(id) ||
        state.localWorldFavoritesList.includes(id)
    );
}

export async function cacheFavoriteWorldDetails(world: unknown) {
    const id = normalizeEntityId(isRecord(world) ? world.id : undefined);
    if (!id) {
        return false;
    }

    if (!isFavoriteWorldId(id)) {
        return false;
    }

    return cacheWorldDetails(world);
}

function reportWorldCacheError(error: unknown) {
    console.warn('Failed to cache favorite world details:', error);
}

export function persistWorldDetails(world: unknown, fallbackWorldId?: unknown) {
    void cacheWorldDetails(world, fallbackWorldId).catch(reportWorldCacheError);
}

export function persistFavoriteWorldDetails(world: unknown) {
    void cacheFavoriteWorldDetails(world).catch(reportWorldCacheError);
}
