import type { InventoryItemRecord } from '@/repositories/vrchatMediaRepository';

import type {
    UserDialogProfileRecord,
    UserDialogProfileSnapshot
} from './userDialogProfileTypes';

const PROFILE_APPEARANCE_FIELDS = [
    'backgroundGradientBottom',
    'backgroundGradientTop',
    'backgroundTemplateId',
    'backgroundTextureId',
    'backgroundType',
    'bannerColor',
    'bannerCustomUrl',
    'bannerType',
    'bannerUrl',
    'hasVrcPlus',
    'iconFrame',
    'iconType',
    'iconUrl',
    'nameplateEffect',
    'profileEffect',
    'themeId',
    'themes',
    'userIcon'
] as const;

export const PROFILE_DECORATION_SLOTS = [
    'iconFrame',
    'profileEffect',
    'nameplateEffect'
] as const;

export type ProfileDecorationSlot = (typeof PROFILE_DECORATION_SLOTS)[number];

export type UserDialogProfileAppearance = Partial<
    Record<ProfileDecorationSlot, InventoryItemRecord>
>;

type ProfileDecorationAssetUrls = {
    animatedUrl: string;
    staticUrl: string;
};

type UserDialogBanner = {
    color: string;
    url: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function normalizeText(value: unknown): string {
    return typeof value === 'string' ? value.trim() : '';
}

export function mergeUserDialogProfileAppearance(
    user: UserDialogProfileSnapshot,
    appearance: unknown,
    targetUserId: string
): UserDialogProfileSnapshot {
    if (!user || !isRecord(appearance)) {
        return user;
    }

    const responseUserId = normalizeText(appearance.id);
    if (responseUserId && responseUserId !== normalizeText(targetUserId)) {
        return user;
    }

    let nextUser = user;
    for (const field of PROFILE_APPEARANCE_FIELDS) {
        if (!Object.prototype.hasOwnProperty.call(appearance, field)) {
            continue;
        }
        if (nextUser === user) {
            nextUser = { ...user };
        }
        nextUser[field] = appearance[field];
    }
    return nextUser;
}

export function normalizeProfileAppearanceColor(value: unknown): string {
    const color = normalizeText(value).replace(/^#/, '');
    return /^[\da-f]{6}$/i.test(color) ? `#${color.toLowerCase()}` : '';
}

export function resolveUserDialogBanner(
    profile: UserDialogProfileRecord
): UserDialogBanner {
    return {
        color: normalizeProfileAppearanceColor(profile.bannerColor),
        url:
            normalizeText(profile.bannerType) === 'color'
                ? ''
                : normalizeText(profile.bannerUrl) ||
                  normalizeText(profile.bannerCustomUrl)
    };
}

export function resolveProfileDecorationAssetUrls(
    item: InventoryItemRecord | null | undefined
): ProfileDecorationAssetUrls {
    const assets = Array.isArray(item?.metadata?.assets)
        ? item.metadata.assets
        : [];
    const assetUrl = (type: string) =>
        normalizeText(assets.find((asset) => asset.type === type)?.url);

    return {
        animatedUrl: assetUrl('mainAnimation'),
        staticUrl: assetUrl('base')
    };
}
