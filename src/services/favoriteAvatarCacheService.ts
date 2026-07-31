import avatarCacheRepository from '@/repositories/avatarCacheRepository';
import { useFavoriteStore } from '@/state/favoriteStore';

function normalizeEntityId(value: unknown) {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeString(value: unknown) {
    return typeof value === 'string' ? value : String(value ?? '');
}

function normalizeReleaseStatus(avatar: any) {
    return normalizeEntityId(avatar?.releaseStatus).toLowerCase();
}

function hasCompleteAvatarSnapshot(avatar: any) {
    const name = normalizeString(avatar?.name).trim();
    const imageUrl =
        normalizeString(avatar?.thumbnailImageUrl).trim() ||
        normalizeString(avatar?.imageUrl).trim();
    return Boolean(name && imageUrl);
}

function canUpsertAvatarSnapshot(avatar: any) {
    return (
        normalizeReleaseStatus(avatar) === 'public' &&
        hasCompleteAvatarSnapshot(avatar)
    );
}

function canInsertMissingAvatarSnapshot(avatar: any) {
    return (
        normalizeReleaseStatus(avatar) !== 'public' &&
        hasCompleteAvatarSnapshot(avatar)
    );
}

function buildAvatarCacheEntry(avatar: any, fallbackAvatarId?: unknown) {
    if (!avatar || typeof avatar !== 'object') {
        return null;
    }

    const id =
        normalizeEntityId(avatar.id) || normalizeEntityId(fallbackAvatarId);
    if (!id) {
        return null;
    }

    if (
        !canUpsertAvatarSnapshot(avatar) &&
        !canInsertMissingAvatarSnapshot(avatar)
    ) {
        return null;
    }

    return {
        id,
        authorId: normalizeEntityId(avatar.authorId),
        authorName: normalizeString(avatar.authorName),
        created_at: normalizeString(avatar.created_at ?? avatar.createdAt),
        description: normalizeString(avatar.description),
        imageUrl: normalizeString(avatar.imageUrl),
        name: normalizeString(avatar.name),
        releaseStatus: normalizeString(avatar.releaseStatus),
        thumbnailImageUrl: normalizeString(avatar.thumbnailImageUrl),
        updated_at: normalizeString(avatar.updated_at ?? avatar.updatedAt),
        version: Number(avatar.version) || 0
    };
}

export async function cacheAvatarDetails(
    avatar: any,
    fallbackAvatarId?: unknown
) {
    const entry = buildAvatarCacheEntry(avatar, fallbackAvatarId);
    if (!entry) {
        return false;
    }

    const canUpsert = canUpsertAvatarSnapshot(avatar);
    if (!canUpsert) {
        const existing = await avatarCacheRepository.getCachedAvatarById(
            entry.id
        );
        if (existing) {
            return false;
        }
    }

    await avatarCacheRepository.addAvatarToCache(entry);
    return true;
}

function isFavoriteAvatarId(id: string) {
    const state = useFavoriteStore.getState();
    return (
        state.favoriteAvatarIds.includes(id) ||
        state.localAvatarFavoritesList.includes(id)
    );
}

export async function cacheFavoriteAvatarDetails(avatar: any) {
    const id = normalizeEntityId(avatar?.id);
    if (!id) {
        return false;
    }

    if (!isFavoriteAvatarId(id)) {
        return false;
    }

    return cacheAvatarDetails(avatar);
}

function reportAvatarCacheError(error: unknown) {
    console.warn('Failed to cache favorite avatar details:', error);
}

export function persistAvatarDetails(avatar: any, fallbackAvatarId?: unknown) {
    void cacheAvatarDetails(avatar, fallbackAvatarId).catch(
        reportAvatarCacheError
    );
}

export function persistFavoriteAvatarDetails(avatar: any) {
    void cacheFavoriteAvatarDetails(avatar).catch(reportAvatarCacheError);
}
