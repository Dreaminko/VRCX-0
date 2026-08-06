import type {
    ColumnSizingState,
    SortingState,
    VisibilityState
} from '@tanstack/react-table';

import {
    getDataTableStorageKey,
    readPersistedTableState,
    safeJsonParse,
    sanitizeTableColumnSizing,
    writePersistedTableState
} from '@/components/data-table/dataTablePersistence';
import { moderationTypes } from '@/shared/constants/moderation';

import type { ModerationRow } from './moderationPageTypes';

export const MODERATION_DEFAULT_PAGE_SIZES = [10, 15, 20, 25, 50, 100];
export const MODERATION_DEFAULT_SORTING = [
    {
        id: 'created',
        desc: true
    }
];
export const MODERATION_COLUMN_IDS = [
    'spacer',
    'created',
    'type',
    'sourceDisplayName',
    'targetDisplayName',
    'action',
    'trailing'
];
const MODERATION_SORTING_COLUMN_IDS = MODERATION_COLUMN_IDS.filter(
    (columnId) =>
        columnId !== 'sourceDisplayName' && columnId !== 'targetDisplayName'
);
export const MODERATION_TYPE_FILTERS_CONFIG_KEY =
    'VRCX_playerModerationTableFilters';

const MODERATION_STORAGE_KEY = getDataTableStorageKey('moderation');
const TYPE_LABELS: Record<string, string> = {
    block: 'Block',
    unblock: 'Unblock',
    mute: 'Mute',
    unmute: 'Unmute',
    interactOn: 'Interact On',
    interactOff: 'Interact Off',
    muteChat: 'Mute Chat',
    unmuteChat: 'Unmute Chat'
};

export function readModerationPersistedState() {
    return readPersistedTableState(MODERATION_STORAGE_KEY);
}

export function writeModerationPersistedState(patch: Record<string, unknown>) {
    writePersistedTableState(MODERATION_STORAGE_KEY, patch);
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

export function resolveModerationTypeLabel(
    type: unknown,
    t: (key: string) => string
) {
    const value = String(type || '');
    if (!value) {
        return '';
    }
    const key = `view.moderation.filters.${value}`;
    const label = t(key);
    return label && label !== key ? label : TYPE_LABELS[value] || value;
}

export function sanitizeModerationSorting(value: unknown): SortingState {
    if (!Array.isArray(value)) {
        return MODERATION_DEFAULT_SORTING;
    }
    const filtered = value.filter(
        (entry): entry is SortingState[number] =>
            isRecord(entry) &&
            typeof entry.id === 'string' &&
            MODERATION_SORTING_COLUMN_IDS.includes(entry.id)
    );
    return filtered.length ? filtered : MODERATION_DEFAULT_SORTING;
}

export function sanitizeModerationPageSizes(value: unknown): number[] {
    if (!Array.isArray(value)) {
        return MODERATION_DEFAULT_PAGE_SIZES;
    }
    const normalized = Array.from(
        new Set(
            value
                .map((entry) => Number.parseInt(String(entry), 10))
                .filter(
                    (entry) =>
                        Number.isFinite(entry) && entry > 0 && entry <= 1000
                )
        )
    ).sort((left, right) => left - right);
    return normalized.length ? normalized : MODERATION_DEFAULT_PAGE_SIZES;
}

export function sanitizeModerationColumnVisibility(
    value: unknown
): VisibilityState {
    const visibility: VisibilityState = {};
    if (!value || typeof value !== 'object') {
        return visibility;
    }
    const source = value as Record<string, unknown>;
    for (const columnId of MODERATION_COLUMN_IDS) {
        if (typeof source[columnId] === 'boolean') {
            visibility[columnId] = source[columnId];
        }
    }
    return visibility;
}

export function sanitizeModerationColumnOrder(value: unknown): string[] {
    if (!Array.isArray(value)) {
        return MODERATION_COLUMN_IDS;
    }
    const orderedColumns = value.filter(
        (columnId): columnId is string =>
            typeof columnId === 'string' &&
            MODERATION_COLUMN_IDS.includes(columnId)
    );
    const missingColumns = MODERATION_COLUMN_IDS.filter(
        (columnId) => !orderedColumns.includes(columnId)
    );
    return [...orderedColumns, ...missingColumns];
}

export function sanitizeModerationColumnSizing(
    value: unknown
): ColumnSizingState {
    return sanitizeTableColumnSizing(value, MODERATION_COLUMN_IDS);
}

export function resolveModerationPageSize(
    candidate: unknown,
    allowed: unknown,
    fallback: unknown = MODERATION_DEFAULT_PAGE_SIZES[1]
): number {
    const pageSizes = Array.isArray(allowed)
        ? allowed.filter(
              (size): size is number => Number.isFinite(size) && size > 0
          )
        : MODERATION_DEFAULT_PAGE_SIZES;
    const fallbackPageSize = pageSizes.length
        ? pageSizes[0]
        : MODERATION_DEFAULT_PAGE_SIZES[0];
    const nearestPageSize = (value: number) =>
        pageSizes.length
            ? pageSizes.reduce((previous, size) =>
                  Math.abs(size - value) < Math.abs(previous - value)
                      ? size
                      : previous
              )
            : fallbackPageSize;
    const parsed = Number.parseInt(String(candidate), 10);
    if (Number.isFinite(parsed) && parsed > 0) {
        return pageSizes.includes(parsed) ? parsed : nearestPageSize(parsed);
    }
    const parsedFallback = Number.parseInt(String(fallback), 10);
    if (pageSizes.includes(parsedFallback)) {
        return parsedFallback;
    }
    return nearestPageSize(Number(fallback) || fallbackPageSize);
}

export function normalizeModerationSelectedTypes(value: unknown): string[] {
    if (!Array.isArray(value)) {
        return [];
    }
    return value.filter(
        (entry): entry is string =>
            typeof entry === 'string' && moderationTypes.includes(entry)
    );
}

export function parseModerationSelectedTypes(value: unknown) {
    return normalizeModerationSelectedTypes(safeJsonParse(value));
}

export function matchesModerationSearch(
    row: ModerationRow,
    searchQuery: unknown
) {
    if (!searchQuery) {
        return true;
    }
    const query = String(searchQuery).trim().toLowerCase();
    if (!query) {
        return true;
    }
    return (
        String(row?.sourceDisplayName ?? '')
            .toLowerCase()
            .includes(query) ||
        String(row?.targetDisplayName ?? '')
            .toLowerCase()
            .includes(query)
    );
}

export function getModerationRowKey(row: ModerationRow) {
    if (row?.id) {
        return `${row.id}:${row.type || ''}`;
    }
    return [
        row?.type || '',
        row?.sourceUserId || '',
        row?.targetUserId || '',
        row?.created || ''
    ].join(':');
}

export function isSameModerationRow(left: ModerationRow, right: ModerationRow) {
    return getModerationRowKey(left) === getModerationRowKey(right);
}
