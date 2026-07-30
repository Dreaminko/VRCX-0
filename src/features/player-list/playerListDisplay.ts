import {
    AppleIcon,
    type LucideIcon,
    MonitorIcon,
    RectangleGogglesIcon
} from 'lucide-react';

import { convertFileUrlToImageUrl } from '@/services/entityMediaService';
import { parseLocation } from '@/shared/utils/location';
import { normalizeString } from '@/shared/utils/string';
import { userStatusIndicatorClassName } from '@/shared/utils/userStatus';

import type { PlayerListRecord, PlayerListRow } from './playerListTypes';

type PlatformMeta = {
    label: string;
    icon: LucideIcon | null;
    className: string;
};

type StatusMeta = {
    badgeVariant: 'default' | 'secondary' | 'outline';
    indicatorClassName: string;
    label: string;
};

type PlayerStatusSource = PlayerListRecord & {
    isCurrentUser?: unknown;
    isFavorite?: unknown;
    isFriend?: unknown;
    location?: unknown;
    status?: unknown;
    statusDescription?: unknown;
};

function isRecord(value: unknown): value is PlayerListRecord {
    return Boolean(value && typeof value === 'object');
}

export function resolvePlatformMeta(platform: unknown): PlatformMeta {
    const normalized = normalizeString(platform).toLowerCase();

    if (
        normalized === 'standalonewindows' ||
        normalized === 'pc' ||
        normalized === 'windows'
    ) {
        return {
            label: 'PC',
            icon: MonitorIcon,
            className: 'text-muted-foreground'
        };
    }

    if (normalized === 'android' || normalized === 'quest') {
        return {
            label: 'Android',
            icon: RectangleGogglesIcon,
            className: 'text-muted-foreground'
        };
    }

    if (normalized === 'ios') {
        return {
            label: 'iOS',
            icon: AppleIcon,
            className: 'text-muted-foreground'
        };
    }

    return {
        label: normalized || '',
        icon: null,
        className: 'text-muted-foreground'
    };
}

function isLivePlayerLocation(location: unknown) {
    const parsed = parseLocation(normalizeString(location));
    return Boolean(
        parsed.worldId &&
        !parsed.isOffline &&
        !parsed.isPrivate &&
        !parsed.isTraveling
    );
}

function normalizePlayerStatus(value: unknown) {
    const normalized = normalizeString(value).toLowerCase();
    if (normalized === 'joinme') {
        return 'join me';
    }
    if (normalized === 'askme') {
        return 'ask me';
    }
    if (normalized === 'offline:offline' || normalized.startsWith('offline ')) {
        return 'offline';
    }
    return normalized;
}

function resolveStatusIndicatorSource(row: PlayerStatusSource) {
    if (!row?.isCurrentUser || !isLivePlayerLocation(row.location)) {
        return row;
    }

    const status = normalizePlayerStatus(row.status);
    return {
        location: row.location,
        state: 'online',
        stateBucket: 'online',
        status: status && status !== 'offline' ? status : 'active'
    };
}

export function resolveStatusMeta(row: PlayerStatusSource): StatusMeta {
    const indicatorClassName = userStatusIndicatorClassName(
        resolveStatusIndicatorSource(row),
        {
            showOffline: true,
            className: 'mr-1'
        }
    );

    if (row.isCurrentUser || row.isFavorite) {
        return {
            badgeVariant: 'default',
            indicatorClassName,
            label: normalizeString(row.statusDescription)
        };
    }

    if (row.isFriend) {
        return {
            badgeVariant: 'secondary',
            indicatorClassName,
            label: normalizeString(row.statusDescription)
        };
    }

    return {
        badgeVariant: 'outline',
        indicatorClassName,
        label: normalizeString(row.statusDescription)
    };
}

export function resolvePlatformMode(
    row: Pick<PlayerListRow, 'inVRMode' | 'platformLabel'>
) {
    if (row?.inVRMode === true) {
        return 'VR';
    }
    if (row?.inVRMode === false) {
        return row?.platformLabel === 'Android' || row?.platformLabel === 'iOS'
            ? 'M'
            : 'D';
    }
    return '';
}

export function languageCodeLabel(languageKey: unknown) {
    const key = normalizeString(languageKey)
        .toLowerCase()
        .replace(/^language_/, '');
    return key ? key.toUpperCase() : '';
}

export function getHomeWorldId(homeLocation: unknown) {
    if (!homeLocation) {
        return '';
    }

    if (typeof homeLocation === 'string') {
        return parseLocation(homeLocation).worldId || homeLocation;
    }

    if (!isRecord(homeLocation)) {
        return '';
    }

    return (
        normalizeString(homeLocation.worldId) ||
        normalizeString(homeLocation.id) ||
        normalizeString(homeLocation.location)
    );
}

export function getWorldImage(world: PlayerListRecord | null | undefined) {
    const imageUrl = normalizeString(
        world?.thumbnailImageUrl || world?.imageUrl || ''
    );
    return imageUrl ? convertFileUrlToImageUrl(imageUrl, 256) : '';
}

export function resolvePlatformBadge(platform: unknown): {
    key: unknown;
    label: unknown;
    icon: LucideIcon | null;
} {
    const normalized = normalizeString(platform).toLowerCase();
    if (
        normalized === 'pc' ||
        normalized === 'standalonewindows' ||
        normalized === 'windows'
    ) {
        return {
            key: 'PC',
            label: 'PC',
            icon: MonitorIcon
        };
    }
    if (normalized === 'quest' || normalized === 'android') {
        return {
            key: 'Quest',
            label: 'Android',
            icon: RectangleGogglesIcon
        };
    }
    if (normalized === 'ios') {
        return {
            key: 'iOS',
            label: 'iOS',
            icon: AppleIcon
        };
    }
    const label = platform || '';
    return {
        key: label,
        label,
        icon: null
    };
}

export function fileAnalysisSizeForPlatform(
    fileAnalysis:
        | Record<string, PlayerListRecord | undefined>
        | null
        | undefined,
    platformKey: unknown
) {
    if (platformKey === 'PC') {
        return normalizeString(fileAnalysis?.standalonewindows?._fileSize);
    }
    if (platformKey === 'Quest' || platformKey === 'Android') {
        return normalizeString(fileAnalysis?.android?._fileSize);
    }
    if (platformKey === 'iOS') {
        return normalizeString(fileAnalysis?.ios?._fileSize);
    }
    return '';
}
