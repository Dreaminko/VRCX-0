import { SCREENSHOT_GALLERY_CONFIG_KEYS } from '@/repositories/configKeys';
import {
    formatDateTimeValue,
    formatIsoDateTime,
    normalizeDateLocale
} from '@/shared/utils/dateTimeFormatters';
import { parseVrchatScreenshotDateFromFileName } from '@/shared/utils/screenshot';
import { useShellStore } from '@/state/shellStore';
import type {
    AuthorDetail,
    PlayerDetail,
    ScreenshotFolderInfo,
    ScreenshotMetadata,
    ScreenshotSearchResult,
    WorldDetail
} from '@/platform/tauri/bindings';

export const SCREENSHOT_METADATA_SEARCH_TYPES = [
    {
        value: 'Player Name',
        index: 0,
        labelKey: 'dialog.screenshot_metadata.search_types.player_name'
    },
    {
        value: 'Player ID',
        index: 1,
        labelKey: 'dialog.screenshot_metadata.search_types.player_id'
    },
    {
        value: 'World Name',
        index: 2,
        labelKey: 'dialog.screenshot_metadata.search_types.world_name'
    },
    {
        value: 'World ID',
        index: 3,
        labelKey: 'dialog.screenshot_metadata.search_types.world_id'
    }
] as const;

export type ScreenshotMetadataSearchType =
    (typeof SCREENSHOT_METADATA_SEARCH_TYPES)[number];

export type ScreenshotSearchSort = {
    asc: boolean;
    key: string;
};

export type ScreenshotSearchRow = Record<string, unknown> & {
    filePath: string;
    dateTime: Date | null;
    playerCount: number;
    dateLabel?: string;
    world: string;
    author?: string;
    resolution?: string;
    match?: string;
};

export type NormalizedScreenshotMetadata = {
    filePath: string;
    fileName: string;
    previousFilePath: string;
    nextFilePath: string;
    resolution: string;
    fileSizeBytes: number;
    dateTime: Date | null;
    world: WorldDetail;
    author: AuthorDetail;
    players: PlayerDetail[];
    note: string;
    application: string;
};

type ScreenshotExtraData = Record<string, unknown> & {
    filePath?: unknown;
    fileName?: unknown;
    previousFilePath?: unknown;
    nextFilePath?: unknown;
    resolution?: unknown;
    fileSizeBytes?: unknown;
    creationDate?: unknown;
};

export type ScreenshotFolderTreeInput = {
    rootPath?: string;
    folders?: Array<
        Partial<ScreenshotFolderInfo> & {
            path: string;
        }
    >;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function stringValue(value: unknown): string {
    return typeof value === 'string' ? value : String(value ?? '');
}

export const DEFAULT_SCREENSHOT_SEARCH_SORT: ScreenshotSearchSort = {
    key: 'dateTime',
    asc: false
};

export const SCREENSHOT_GALLERY_FOLDER_CONFIG_KEY =
    SCREENSHOT_GALLERY_CONFIG_KEYS.folder;
export const SCREENSHOT_GALLERY_SCROLL_CONFIG_KEY =
    SCREENSHOT_GALLERY_CONFIG_KEYS.scrollPositions;
export const SCREENSHOT_GALLERY_SCROLL_SAVE_DELAY_MS = 500;
export const MAX_SCREENSHOT_GALLERY_SCROLL_POSITIONS = 100;
export const MAX_SCREENSHOT_GALLERY_SCROLL_TOP = 50_000_000;

export function normalizeGalleryScrollTop(value: unknown): number {
    const numeric = Number(value);
    if (!Number.isFinite(numeric)) {
        return 0;
    }
    return Math.min(
        MAX_SCREENSHOT_GALLERY_SCROLL_TOP,
        Math.max(0, Math.round(numeric))
    );
}

export function normalizeGalleryScrollPositions(
    value: unknown
): Map<string, number> {
    const entries =
        value && typeof value === 'object' && !Array.isArray(value)
            ? Object.entries(value)
            : [];
    const positions = new Map();

    for (const [path, scrollTop] of entries) {
        if (!path || typeof path !== 'string') {
            continue;
        }
        positions.set(path, normalizeGalleryScrollTop(scrollTop));
        if (positions.size >= MAX_SCREENSHOT_GALLERY_SCROLL_POSITIONS) {
            break;
        }
    }

    return positions;
}

export function serializeGalleryScrollPositions(
    positions: Map<unknown, unknown>
): Record<string, number> {
    const result: Record<string, number> = {};
    const entries = Array.from(positions.entries())
        .filter(
            (entry): entry is [unknown, unknown] =>
                Array.isArray(entry) && Boolean(entry[0])
        )
        .slice(-MAX_SCREENSHOT_GALLERY_SCROLL_POSITIONS);
    for (const [path, scrollTop] of entries) {
        result[String(path)] = normalizeGalleryScrollTop(scrollTop);
    }
    return result;
}

export function getGalleryFolderPathSet(
    folderTree: ScreenshotFolderTreeInput | null | undefined
) {
    return new Set(
        (folderTree?.folders ?? [])
            .map((folder) => folder.path)
            .filter(Boolean)
    );
}

export function getFolderLatestModifiedAt(
    folder: Partial<ScreenshotFolderInfo>
) {
    return Number(folder?.latestModifiedAt) || 0;
}

export function resolveGalleryFolder(
    folderTree: ScreenshotFolderTreeInput | null | undefined,
    preferredFolders: unknown
) {
    const folders = folderTree?.folders ?? [];
    const preferredList = Array.isArray(preferredFolders)
        ? preferredFolders
        : [preferredFolders];
    for (const preferredFolder of preferredList) {
        if (
            preferredFolder &&
            folders.some((folder) => folder.path === preferredFolder)
        ) {
            return preferredFolder;
        }
    }
    const latestFolder = folders
        .filter((folder) => Number(folder.imageCount) > 0)
        .sort(
            (left, right) =>
                getFolderLatestModifiedAt(right) -
                    getFolderLatestModifiedAt(left) ||
                String(right.path || '').localeCompare(String(left.path || ''))
        )[0];
    return latestFolder?.path || folderTree?.rootPath || folders[0]?.path || '';
}

export function normalizeDroppedFilePath(value: unknown) {
    const text = String(value || '')
        .split(/\r?\n/)
        .map((line) => line.trim())
        .find(Boolean);

    if (!text) {
        return '';
    }

    if (text.startsWith('file://')) {
        try {
            const url = new URL(text);
            const pathname = decodeURIComponent(url.pathname);
            return /^[A-Za-z]:/.test(pathname.slice(1))
                ? pathname.slice(1)
                : pathname;
        } catch {
            return text;
        }
    }

    return text;
}

export function getDroppedScreenshotPath(event: {
    dataTransfer?: {
        files?: ArrayLike<{ path?: string; webkitRelativePath?: string }>;
        getData?: (format: string) => string;
    } | null;
}) {
    const file = event.dataTransfer?.files?.[0] || null;
    const filePath = file?.path || file?.webkitRelativePath || '';
    if (filePath) {
        return filePath;
    }

    return normalizeDroppedFilePath(
        event.dataTransfer?.getData?.('text/uri-list') ||
            event.dataTransfer?.getData?.('text/plain') ||
            ''
    );
}

export function getScreenshotSearchSortValue(
    row: ScreenshotSearchRow,
    key: string
) {
    if (key === 'dateTime') {
        return row?.dateTime?.getTime?.() ?? 0;
    }
    if (key === 'playerCount') {
        return Number(row?.playerCount) || 0;
    }
    return String(row?.[key] || '').toLowerCase();
}

export function sortScreenshotSearchRows(
    rows: ScreenshotSearchRow[],
    sort: ScreenshotSearchSort
): ScreenshotSearchRow[] {
    const sortKey = sort?.key || DEFAULT_SCREENSHOT_SEARCH_SORT.key;
    const direction = sort?.asc ? 1 : -1;
    return [...rows].sort((left, right) => {
        const leftValue = getScreenshotSearchSortValue(left, sortKey);
        const rightValue = getScreenshotSearchSortValue(right, sortKey);
        if (leftValue < rightValue) {
            return -1 * direction;
        }
        if (leftValue > rightValue) {
            return 1 * direction;
        }
        const leftTime = left?.dateTime?.getTime?.() ?? 0;
        const rightTime = right?.dateTime?.getTime?.() ?? 0;
        return rightTime - leftTime;
    });
}

export function formatScreenshotBytes(bytes: unknown): string {
    const sizeInBytes = Number(bytes);
    if (!Number.isFinite(sizeInBytes) || sizeInBytes <= 0) {
        return '';
    }

    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let size = sizeInBytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
        size /= 1024;
        unitIndex += 1;
    }

    const precision = size >= 100 || unitIndex === 0 ? 0 : 1;
    return `${size.toFixed(precision)} ${units[unitIndex]}`;
}

export function formatScreenshotDateTime(
    value: unknown,
    locale: unknown = undefined
) {
    if (!value) {
        return '—';
    }

    const date =
        value instanceof Date
            ? value
            : new Date(
                  typeof value === 'string' || typeof value === 'number'
                      ? value
                      : String(value)
              );
    if (Number.isNaN(date.getTime())) {
        return '—';
    }

    const {
        dateHour12,
        dateIsoFormat,
        locale: appLocale
    } = useShellStore.getState();

    if (dateIsoFormat) {
        return formatIsoDateTime(date);
    }

    return formatDateTimeValue(
        date,
        {
            dateStyle: 'medium',
            timeStyle: 'short'
        },
        {
            locale: normalizeDateLocale(locale || appLocale, 'en'),
            hour12: Boolean(dateHour12),
            fallback: '—'
        }
    );
}

export function getFileNameFromPath(path: unknown) {
    return (
        String(path || '')
            .split(/[\\/]/)
            .filter(Boolean)
            .at(-1) || ''
    );
}

export function resolveScreenshotMetadataDate(
    metadata: Partial<ScreenshotMetadata>,
    extra: ScreenshotExtraData,
    fileName: unknown
) {
    if (metadata?.timestamp) {
        const parsed = Date.parse(metadata.timestamp);
        if (Number.isFinite(parsed)) {
            return new Date(parsed);
        }
    }

    const fileNameTimestamp = parseVrchatScreenshotDateFromFileName(fileName);
    if (fileNameTimestamp !== null && Number.isFinite(fileNameTimestamp)) {
        return new Date(fileNameTimestamp);
    }

    if (extra?.creationDate) {
        const parsed = Date.parse(stringValue(extra.creationDate));
        if (Number.isFinite(parsed)) {
            return new Date(parsed);
        }
    }

    return null;
}

export function normalizeScreenshotMetadata(
    metadata: unknown,
    extra: unknown = {}
): NormalizedScreenshotMetadata {
    const source = isRecord(metadata) ? metadata : {};
    const extraData: ScreenshotExtraData = isRecord(extra) ? extra : {};
    const fileName =
        stringValue(extraData.fileName) ||
        getFileNameFromPath(extraData.filePath || source.sourceFile);
    const typedMetadata: Partial<ScreenshotMetadata> = {
        timestamp:
            typeof source.timestamp === 'string' ? source.timestamp : null,
        sourceFile:
            typeof source.sourceFile === 'string' ? source.sourceFile : null
    };
    const dateTime = resolveScreenshotMetadataDate(
        typedMetadata,
        extraData,
        fileName
    );
    const world = isRecord(source.world)
        ? {
              id: stringValue(source.world.id) || undefined,
              name: stringValue(source.world.name) || undefined,
              instanceId: stringValue(source.world.instanceId) || undefined
          }
        : {};
    const author = isRecord(source.author)
        ? {
              id: stringValue(source.author.id) || undefined,
              displayName:
                  stringValue(source.author.displayName) || undefined
          }
        : {};
    const players = Array.isArray(source.players)
        ? source.players.flatMap((player): PlayerDetail[] => {
              if (!isRecord(player)) {
                  return [];
              }
              return [
                  {
                      id: stringValue(player.id) || undefined,
                      displayName:
                          stringValue(player.displayName) || undefined
                  }
              ];
          })
        : [];

    return {
        filePath: stringValue(extraData.filePath || source.sourceFile),
        fileName,
        previousFilePath: stringValue(extraData.previousFilePath),
        nextFilePath: stringValue(extraData.nextFilePath),
        resolution: stringValue(extraData.resolution),
        fileSizeBytes: Number(extraData.fileSizeBytes) || 0,
        dateTime,
        world,
        author,
        players,
        note: stringValue(source.note),
        application: stringValue(source.application)
    };
}

export function normalizeScreenshotSearchResult(
    result: Omit<ScreenshotSearchResult, 'metadata'> & {
        metadata: ScreenshotMetadata | null;
    }
) {
    const width = Number(result?.width) || 0;
    const height = Number(result?.height) || 0;
    return normalizeScreenshotMetadata(result?.metadata ?? {}, {
        filePath: result?.filePath || '',
        fileName: result?.fileName || '',
        fileSizeBytes: result?.fileSizeBytes ?? 0,
        creationDate: result?.creationDate || '',
        resolution: width > 0 && height > 0 ? `${width}x${height}` : ''
    });
}

export function buildScreenshotSearchRow(
    normalized: NormalizedScreenshotMetadata,
    selectedSearchType: ScreenshotMetadataSearchType,
    query: unknown,
    locale: unknown = undefined
): ScreenshotSearchRow {
    let match = '';
    if (selectedSearchType?.index === 0) {
        const normalizedQuery = String(query || '').toLowerCase();
        const hits = normalized.players
            .filter((player) =>
                String(player.displayName || '')
                    .toLowerCase()
                    .includes(normalizedQuery)
            )
            .map((player) => player.displayName || '');
        match = hits.join(', ');
    } else if (selectedSearchType?.index === 1) {
        match =
            normalized.players.find((player) => player.id === query)
                ?.displayName || '';
    }

    return {
        filePath: normalized.filePath,
        dateTime: normalized.dateTime,
        dateLabel: formatScreenshotDateTime(normalized.dateTime, locale),
        world: normalized.world?.name || '—',
        author: normalized.author?.displayName || '—',
        playerCount: normalized.players.length,
        resolution: normalized.resolution || '—',
        match: match || '—'
    };
}

export function sortScreenshotRowsByNewest(
    rows: Array<ScreenshotSearchRow | null>
): ScreenshotSearchRow[] {
    return (Array.isArray(rows) ? rows : [])
        .filter((row): row is ScreenshotSearchRow => Boolean(row))
        .sort((left, right) => {
            const leftTime = left?.dateTime?.getTime?.() ?? 0;
            const rightTime = right?.dateTime?.getTime?.() ?? 0;
            return rightTime - leftTime;
        });
}
