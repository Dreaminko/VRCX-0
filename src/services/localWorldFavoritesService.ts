import { commands } from '@/platform/tauri/bindings';
import type { FavoriteGroupMap } from '@/state/favoriteStoreTypes';

export interface LocalWorldFavoritesSnapshot {
    favoritesByGroup: FavoriteGroupMap;
    groupNames: string[];
}

function normalize(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

export async function loadLocalWorldFavoritesSnapshot(): Promise<LocalWorldFavoritesSnapshot> {
    const snapshot = await commands.appFavoriteLocalSnapshot('world');
    const favoritesByGroup: FavoriteGroupMap = {};
    for (const groupName of snapshot.groupNames) {
        const normalizedGroupName = normalize(groupName);
        if (normalizedGroupName) {
            favoritesByGroup[normalizedGroupName] = [];
        }
    }

    for (const row of snapshot.favorites) {
        const worldId = normalize(row.worldId);
        const groupName = normalize(row.groupName) || 'Favorites';
        if (!worldId) {
            continue;
        }
        const ids = favoritesByGroup[groupName] || [];
        if (!ids.includes(worldId)) {
            favoritesByGroup[groupName] = [worldId, ...ids];
        }
    }

    if (Object.keys(favoritesByGroup).length === 0) {
        favoritesByGroup.Favorites = [];
    }

    return {
        favoritesByGroup,
        groupNames: Object.keys(favoritesByGroup).sort()
    };
}
