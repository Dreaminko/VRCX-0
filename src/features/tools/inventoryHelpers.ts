import { toast } from 'sonner';

import type {
    InventoryItemRecord,
    MediaFileRecord
} from '@/repositories/mediaRepository';
import { emojiAnimationStyleList } from '@/shared/constants/emoji';
import {
    MAX_IMAGE_UPLOAD_BYTES,
    validateImageUploadFile
} from '@/shared/utils/imageUpload';

import {
    getGalleryGridDensityConfig,
    sanitizeGalleryGridDensity
} from './galleryDensity';

export { MAX_IMAGE_UPLOAD_BYTES };

export const INVENTORY_GRID_DENSITY_STORAGE_KEY = 'VRCX_InventoryGridDensity';

export const CATEGORY_ORDER = [
    'emojis',
    'stickers',
    'items',
    'cosmetics'
] as const;
export type InventoryCategory = (typeof CATEGORY_ORDER)[number];
export type InventorySource = 'file' | 'inventory' | 'empty';
export type InventoryUploadTarget = 'emojis' | 'stickers';
export type InventoryTabDefinition = {
    key: string;
    labelKey: string;
    source: InventorySource;
    fileTags?: string[];
    uploadTarget?: InventoryUploadTarget;
    params: Record<string, string | boolean>;
};
export type InventoryCategoryDefinition = {
    labelKey: string;
    tabs: InventoryTabDefinition[];
};
export const INITIAL_INVENTORY_SUB_TABS = Object.freeze({
    emojis: 'custom',
    stickers: 'custom',
    items: 'all',
    cosmetics: 'profile-decorations'
});

const PROFILE_DECORATION_ITEM_TYPES = [
    'iconFrame',
    'profileEffect',
    'nameplateEffect'
] as const;
const PROFILE_DECORATION_TYPES_PARAM = PROFILE_DECORATION_ITEM_TYPES.join(',');
type ProfileDecorationItemType = (typeof PROFILE_DECORATION_ITEM_TYPES)[number];
export type ProfileDecorationMutation = {
    action: 'equip' | 'unequip';
    equipSlot: ProfileDecorationItemType;
    inventoryId: string;
};

const PROFILE_DECORATION_TYPE_LABEL_KEYS: Record<
    ProfileDecorationItemType,
    string
> = {
    iconFrame: 'dialog.inventory.icon_frame',
    profileEffect: 'dialog.inventory.profile_effect',
    nameplateEffect: 'dialog.inventory.nameplate_effect'
};

const PROFILE_DECORATION_PREVIEW_ASSET_TYPES = [
    'mainAnimation',
    'introAnimation',
    'base'
] as const;

function isProfileDecorationItemType(
    value: unknown
): value is ProfileDecorationItemType {
    return (
        typeof value === 'string' &&
        PROFILE_DECORATION_ITEM_TYPES.some((itemType) => itemType === value)
    );
}

export const CATEGORY_DEFINITIONS: Record<
    InventoryCategory,
    InventoryCategoryDefinition
> = {
    emojis: {
        labelKey: 'dialog.inventory.emojis',
        tabs: [
            {
                key: 'custom',
                labelKey: 'dialog.inventory.custom',
                source: 'file',
                fileTags: ['emoji', 'emojianimated'],
                uploadTarget: 'emojis',
                params: {}
            },
            {
                key: 'exclusive',
                labelKey: 'dialog.inventory.exclusive',
                source: 'inventory',
                params: {
                    types: 'emoji',
                    notFlags: 'ugc',
                    archived: false
                }
            },
            {
                key: 'archived',
                labelKey: 'dialog.inventory.archived',
                source: 'inventory',
                params: {
                    types: 'emoji',
                    archived: true
                }
            }
        ]
    },
    stickers: {
        labelKey: 'dialog.inventory.stickers',
        tabs: [
            {
                key: 'custom',
                labelKey: 'dialog.inventory.custom',
                source: 'file',
                fileTags: ['sticker'],
                uploadTarget: 'stickers',
                params: {}
            },
            {
                key: 'exclusive',
                labelKey: 'dialog.inventory.exclusive',
                source: 'inventory',
                params: {
                    types: 'sticker',
                    notFlags: 'ugc',
                    archived: false
                }
            },
            {
                key: 'archived',
                labelKey: 'dialog.inventory.archived',
                source: 'inventory',
                params: {
                    types: 'sticker',
                    archived: true
                }
            }
        ]
    },
    items: {
        labelKey: 'dialog.inventory.items',
        tabs: [
            {
                key: 'all',
                labelKey: 'dialog.inventory.all_items',
                source: 'inventory',
                params: {
                    types: 'bundle,prop',
                    notFlags: 'ugc',
                    archived: false
                }
            },
            {
                key: 'archived',
                labelKey: 'dialog.inventory.archived',
                source: 'inventory',
                params: {
                    types: 'bundle,prop',
                    archived: true
                }
            }
        ]
    },
    cosmetics: {
        labelKey: 'dialog.inventory.cosmetics',
        tabs: [
            {
                key: 'profile-decorations',
                labelKey: 'dialog.inventory.profile_decorations',
                source: 'inventory',
                params: {
                    types: PROFILE_DECORATION_TYPES_PARAM,
                    notFlags: 'ugc',
                    archived: false
                }
            },
            {
                key: 'drones',
                labelKey: 'dialog.inventory.drones',
                source: 'inventory',
                params: {
                    types: 'droneskin',
                    notFlags: 'ugc',
                    archived: false
                }
            },
            {
                key: 'portals',
                labelKey: 'dialog.inventory.portals',
                source: 'inventory',
                params: {
                    types: 'portalskin',
                    notFlags: 'ugc',
                    archived: false
                }
            },
            {
                key: 'warp-effects',
                labelKey: 'dialog.inventory.warp_effects',
                source: 'inventory',
                params: {
                    types: 'warpeffect',
                    notFlags: 'ugc',
                    archived: false
                }
            },
            {
                key: 'loading-screens',
                labelKey: 'dialog.inventory.loading_screens',
                source: 'empty',
                params: {}
            },
            {
                key: 'archived',
                labelKey: 'dialog.inventory.archived',
                source: 'inventory',
                params: {
                    types: `droneskin,portalskin,warpeffect,${PROFILE_DECORATION_TYPES_PARAM}`,
                    archived: true
                }
            }
        ]
    }
};

export function scopeKey(category: string, tab: string) {
    return `${category}:${tab}`;
}

export function readGridDensityPreference() {
    if (typeof window === 'undefined') {
        return sanitizeGalleryGridDensity();
    }
    try {
        return sanitizeGalleryGridDensity(
            window.localStorage.getItem(INVENTORY_GRID_DENSITY_STORAGE_KEY)
        );
    } catch {
        return sanitizeGalleryGridDensity();
    }
}

export function writeGridDensityPreference(value: string) {
    if (typeof window === 'undefined') {
        return;
    }
    try {
        window.localStorage.setItem(INVENTORY_GRID_DENSITY_STORAGE_KEY, value);
    } catch {
        // no-op
    }
}

export function getInventoryGridDensityConfig(gridDensity: unknown) {
    return getGalleryGridDensityConfig(gridDensity);
}

export function sanitizeInventoryGridDensity(nextValue: unknown) {
    return sanitizeGalleryGridDensity(nextValue);
}

export function getLatestFileUrl(file: Pick<MediaFileRecord, 'versions'>) {
    const versions = Array.isArray(file?.versions) ? file.versions : [];
    return versions.at(-1)?.file?.url ?? '';
}

export function getUsefulDisplayName(
    file: Partial<Pick<MediaFileRecord, 'displayName' | 'name' | 'id'>>
) {
    const displayName = String(file?.displayName || '').trim();
    const name = String(file?.name || '').trim();
    const id = String(file?.id || '').trim();
    const visibleName = displayName || name;

    if (
        !visibleName ||
        visibleName === id ||
        /^file_[\w-]+_blob$/i.test(visibleName)
    ) {
        return '';
    }

    return visibleName;
}

type InventoryDisplayRecord = Record<string, unknown> & {
    id?: string;
    imageUrl?: string;
    thumbnailUrl?: string;
    name?: string;
    description?: string;
    displayName?: string;
    itemType?: string;
    type?: string;
    isArchived?: boolean;
    archived?: boolean;
    item?: InventoryDisplayRecord | null;
    template?: InventoryDisplayRecord | null;
    metadata?: (Record<string, unknown> & { imageUrl?: string }) | null;
};

export function resolveInventoryImageUrl(item: InventoryDisplayRecord) {
    return String(
        item?.imageUrl ||
            item?.thumbnailUrl ||
            item?.item?.imageUrl ||
            item?.item?.thumbnailUrl ||
            item?.template?.imageUrl ||
            item?.template?.thumbnailUrl ||
            item?.metadata?.imageUrl ||
            ''
    );
}

export function resolveInventoryName(item: InventoryDisplayRecord) {
    return String(
        item?.name ||
            item?.item?.name ||
            item?.template?.name ||
            item?.displayName ||
            item?.id ||
            ''
    );
}

export function resolveInventoryDescription(item: InventoryDisplayRecord) {
    return String(
        item?.description ||
            item?.item?.description ||
            item?.template?.description ||
            ''
    );
}

export function resolveInventoryType(item: InventoryDisplayRecord) {
    return String(item?.itemType || item?.type || item?.item?.type || '');
}

export function resolveProfileDecorationTypeLabelKey(
    itemType: unknown
): string | null {
    if (!isProfileDecorationItemType(itemType)) {
        return null;
    }
    return PROFILE_DECORATION_TYPE_LABEL_KEYS[itemType];
}

export function isEquippedProfileDecoration(
    item: InventoryItemRecord
): boolean {
    return (
        isProfileDecorationItemType(item.itemType) &&
        item.equipSlot === item.itemType
    );
}

export function resolveProfileDecorationMutation(
    item: InventoryItemRecord,
    currentUserId: unknown
): ProfileDecorationMutation | null {
    const inventoryId = item.id?.trim() ?? '';
    const normalizedCurrentUserId =
        typeof currentUserId === 'string' ? currentUserId.trim() : '';
    const holderId = item.holderId?.trim() ?? '';
    if (
        !inventoryId.startsWith('inv_') ||
        !normalizedCurrentUserId ||
        !isProfileDecorationItemType(item.itemType) ||
        !item.equipSlots?.includes(item.itemType) ||
        !item.flags?.includes('equippable') ||
        isArchivedInventoryItem(item) ||
        (holderId && holderId !== normalizedCurrentUserId)
    ) {
        return null;
    }

    return {
        action: item.equipSlot === item.itemType ? 'unequip' : 'equip',
        equipSlot: item.itemType,
        inventoryId
    };
}

export function resolveProfileDecorationPreviewUrl(
    item: InventoryItemRecord
): string {
    const assets = Array.isArray(item.metadata?.assets)
        ? item.metadata.assets
        : [];
    for (const assetType of PROFILE_DECORATION_PREVIEW_ASSET_TYPES) {
        const asset = assets.find(
            (candidate) =>
                candidate.type === assetType &&
                typeof candidate.url === 'string' &&
                candidate.url.trim()
        );
        const url = typeof asset?.url === 'string' ? asset.url.trim() : '';
        if (url) {
            return url;
        }
    }
    return resolveInventoryImageUrl(item);
}

export function isArchivedInventoryItem(item: InventoryDisplayRecord) {
    return Boolean(item?.isArchived || item?.archived);
}

export function resolveEmojiStyleName(rawValue: unknown) {
    const normalizedValue = String(rawValue || '').toLowerCase();
    const match = Object.keys(emojiAnimationStyleList).find(
        (styleName) => styleName.toLowerCase() === normalizedValue
    );
    return match || 'Stop';
}

export type EmojiUploadSettings = {
    isAnimated: boolean;
    animationStyle: string;
    fps: number;
    frames: number;
    loopPingPong: boolean;
};

export function parseEmojiUploadSettings(
    fileName: unknown,
    currentSettings: Partial<EmojiUploadSettings> = {}
): EmojiUploadSettings {
    const next: EmojiUploadSettings = {
        isAnimated: Boolean(currentSettings.isAnimated),
        animationStyle: currentSettings.animationStyle || 'Stop',
        fps: Number(currentSettings.fps) || 15,
        frames: Number(currentSettings.frames) || 4,
        loopPingPong: Boolean(currentSettings.loopPingPong)
    };
    for (const value of String(fileName || '')
        .replace(/\.[^/.]+$/, '')
        .split('_')) {
        if (value.endsWith('animationStyle')) {
            next.isAnimated = false;
            next.animationStyle = resolveEmojiStyleName(
                value.replace('animationStyle', '')
            );
        } else if (value.endsWith('frames')) {
            const frames = Number.parseInt(value.replace('frames', ''), 10);
            if (Number.isFinite(frames)) {
                next.isAnimated = true;
                next.frames = Math.min(64, Math.max(2, frames));
            }
        } else if (value.endsWith('fps')) {
            const fps = Number.parseInt(value.replace('fps', ''), 10);
            if (Number.isFinite(fps)) {
                next.fps = Math.min(64, Math.max(1, fps));
            }
        } else if (value.endsWith('loopStyle')) {
            next.loopPingPong =
                value.replace('loopStyle', '').toLowerCase() === 'pingpong';
        }
    }
    return next;
}

export function validateImageFile(file: Blob, t: (key: string) => string) {
    const validation = validateImageUploadFile(file, {
        maxSize: MAX_IMAGE_UPLOAD_BYTES
    });
    if (!validation.ok) {
        toast.error(
            validation.reason === 'too_large'
                ? t('message.file.too_large')
                : t('message.file.not_image')
        );
        return false;
    }
    return true;
}
