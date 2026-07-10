import type {
    FavoriteTransferInput,
    FavoriteTransferItemResult,
    FavoriteTransferLocation
} from '@/platform/tauri/bindings';

import { favoriteGroupType, normalizeFavoriteEntityId } from './favoritesItems';
import type {
    FavoriteGroup,
    FavoriteItem,
    FavoriteKind,
    FavoriteSource
} from './favoritesTypes';

type BuildFavoriteTransferTargetsInput = {
    remoteGroups: FavoriteGroup[];
    localGroups: FavoriteGroup[];
    selectedSource: FavoriteSource;
    selectedGroupKey: string;
};

type BuildFavoriteTransferInputOptions = {
    endpoint: string;
    kind: FavoriteKind;
    sourceGroup: FavoriteGroup;
    targetGroup: FavoriteGroup;
    selectedItems: FavoriteItem[];
};

type BuildFavoriteTransferFailureDescriptionInput = {
    results: FavoriteTransferItemResult[];
    selectedItems: FavoriteItem[];
    fallbackMessage: string;
    maxItems?: number;
};

function favoriteTransferLocation(
    source: FavoriteSource
): FavoriteTransferLocation {
    if (source === 'remote') {
        return 'remote';
    }
    return 'local';
}

function remoteGroupName(group: FavoriteGroup): string {
    return (
        normalizeFavoriteEntityId(group.name) ||
        normalizeFavoriteEntityId(group.key).split(':').pop() ||
        ''
    );
}

function transferGroupName(group: FavoriteGroup): string {
    return group.source === 'remote'
        ? remoteGroupName(group)
        : normalizeFavoriteEntityId(group.key);
}

function buildTransferItem(item: FavoriteItem) {
    return {
        key: item.key,
        entityId: item.id,
        entity: item.seedData ?? null
    };
}

export function buildFavoriteTransferSuccessfulKeys(
    results: FavoriteTransferItemResult[]
): Set<string> {
    return new Set(
        results
            .filter((result) => result.status !== 'failed')
            .map((result) => result.key)
            .filter(Boolean)
    );
}

export function buildFavoriteTransferFailureDescription({
    results,
    selectedItems,
    fallbackMessage,
    maxItems = 5
}: BuildFavoriteTransferFailureDescriptionInput): string {
    const itemsByKey = new Map(selectedItems.map((item) => [item.key, item]));
    return results
        .filter((result) => result.status === 'failed')
        .slice(0, maxItems)
        .map((result) => {
            const item = itemsByKey.get(result.key);
            const label = item?.title || result.entityId || result.key;
            const message = result.message || fallbackMessage;
            return `${label} [${result.stage}]: ${message}`;
        })
        .join('\n');
}

export function isFavoriteMoveTargetOverCapacity(
    target: FavoriteGroup,
    additionalCount: number
): boolean {
    if (typeof target.capacity !== 'number') {
        return false;
    }
    const currentCount = typeof target.count === 'number' ? target.count : 0;
    return currentCount + additionalCount > target.capacity;
}

function favoriteItemSourceGroupBatchKey(item: FavoriteItem): string {
    return `${item.source}:${item.groupKey ?? ''}`;
}

export function groupFavoriteItemsBySourceGroup(
    items: FavoriteItem[]
): FavoriteItem[][] {
    const buckets = new Map<string, FavoriteItem[]>();
    for (const item of items) {
        const batchKey = favoriteItemSourceGroupBatchKey(item);
        const bucket = buckets.get(batchKey);
        if (bucket) {
            bucket.push(item);
        } else {
            buckets.set(batchKey, [item]);
        }
    }
    return Array.from(buckets.values());
}

export function resolveFavoriteSourceGroup({
    item,
    remoteGroups,
    localGroups
}: {
    item: FavoriteItem;
    remoteGroups: FavoriteGroup[];
    localGroups: FavoriteGroup[];
}): FavoriteGroup {
    const candidates = item.source === 'remote' ? remoteGroups : localGroups;
    const matched = candidates.find((group) => group.key === item.groupKey);
    if (matched) {
        return matched;
    }
    return {
        key: item.groupKey || '',
        source: item.source,
        label: item.groupLabel || item.groupKey || ''
    };
}

export function buildFavoriteTransferTargets({
    remoteGroups,
    localGroups,
    selectedSource,
    selectedGroupKey
}: BuildFavoriteTransferTargetsInput): FavoriteGroup[] {
    return [...remoteGroups, ...localGroups].filter(
        (group) =>
            group.source !== 'history' &&
            !(group.source === selectedSource && group.key === selectedGroupKey)
    );
}

export function buildFavoriteTransferInput({
    endpoint,
    kind,
    sourceGroup,
    targetGroup,
    selectedItems
}: BuildFavoriteTransferInputOptions): FavoriteTransferInput {
    return {
        endpoint,
        kind,
        source: {
            location: favoriteTransferLocation(sourceGroup.source),
            group: transferGroupName(sourceGroup)
        },
        target: {
            location: favoriteTransferLocation(targetGroup.source),
            group: transferGroupName(targetGroup),
            favoriteType:
                targetGroup.source === 'remote'
                    ? favoriteGroupType(kind, targetGroup)
                    : ''
        },
        items: selectedItems.map((item) => buildTransferItem(item))
    };
}
