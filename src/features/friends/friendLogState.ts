import type { SortingState } from '@tanstack/react-table';

import {
    getDataTableStorageKey,
    readPersistedTableState,
    safeJsonParse,
    sanitizeTableColumnSizing,
    writePersistedTableState
} from '@/components/data-table/dataTablePersistence';

import {
    FRIEND_LOG_TYPES,
    type FriendLogType
} from './components/FriendLogViewParts';

export const DEFAULT_PAGE_SIZES = [10, 15, 20, 25, 50, 100];
export const COLUMN_IDS = [
    'spacer',
    'created_at',
    'type',
    'displayName',
    'action',
    'trailing'
];
const SORTING_COLUMN_IDS = COLUMN_IDS.filter(
    (columnId) => columnId !== 'displayName'
);

const DEFAULT_SORTING: SortingState = [];
const STORAGE_KEY = getDataTableStorageKey('friendLog');

export function readPersistedState() {
    return readPersistedTableState(STORAGE_KEY);
}

export function writePersistedState(patch: Record<string, unknown>) {
    writePersistedTableState(STORAGE_KEY, patch);
}

export function sanitizeSorting(value: unknown): SortingState {
    if (!Array.isArray(value)) {
        return DEFAULT_SORTING;
    }

    return value.filter((entry): entry is SortingState[number] => {
        if (!entry || typeof entry !== 'object') {
            return false;
        }
        const candidate = Object.fromEntries(Object.entries(entry));
        return (
            typeof candidate.id === 'string' &&
            SORTING_COLUMN_IDS.includes(candidate.id)
        );
    });
}

export function sanitizePageSizes(value: unknown): number[] {
    if (!Array.isArray(value)) {
        return DEFAULT_PAGE_SIZES;
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

    return normalized.length ? normalized : DEFAULT_PAGE_SIZES;
}

export function sanitizeColumnVisibility(value: unknown) {
    const visibility: Record<string, boolean> = {};
    if (!value || typeof value !== 'object') {
        return visibility;
    }

    const source = Object.fromEntries(Object.entries(value));
    for (const columnId of COLUMN_IDS) {
        if (typeof source[columnId] === 'boolean') {
            visibility[columnId] = source[columnId];
        }
    }

    return visibility;
}

export function sanitizeColumnOrder(value: unknown): string[] {
    if (!Array.isArray(value)) {
        return COLUMN_IDS;
    }

    const orderedColumns = value.filter(
        (columnId): columnId is string =>
            typeof columnId === 'string' && COLUMN_IDS.includes(columnId)
    );
    const missingColumns = COLUMN_IDS.filter(
        (columnId) => !orderedColumns.includes(columnId)
    );
    return [...orderedColumns, ...missingColumns];
}

export function sanitizeColumnSizing(value: unknown) {
    return sanitizeTableColumnSizing(value, COLUMN_IDS);
}

export function resolvePageSize(
    candidate: unknown,
    allowed: unknown,
    fallback: number = DEFAULT_PAGE_SIZES[1]
) {
    const pageSizes = Array.isArray(allowed)
        ? allowed.filter(
              (size): size is number =>
                  typeof size === 'number' && Number.isFinite(size) && size > 0
          )
        : DEFAULT_PAGE_SIZES;
    const fallbackPageSize = pageSizes.length
        ? pageSizes[0]
        : DEFAULT_PAGE_SIZES[0];
    const nearestPageSize = (value: number) =>
        pageSizes.length
            ? pageSizes.reduce((previous, size) =>
                  Math.abs(size - value) < Math.abs(previous - value)
                      ? size
                      : previous
              )
            : fallbackPageSize;
    const parsed = Number.parseInt(String(candidate ?? ''), 10);
    if (Number.isFinite(parsed) && parsed > 0) {
        return pageSizes.includes(parsed) ? parsed : nearestPageSize(parsed);
    }

    if (pageSizes.includes(fallback)) {
        return fallback;
    }

    return nearestPageSize(Number(fallback) || fallbackPageSize);
}

export function parseTypeFilters(value: unknown): FriendLogType[] {
    const parsed = safeJsonParse(value);
    if (!Array.isArray(parsed)) {
        return [];
    }

    return parsed.filter(
        (entry): entry is FriendLogType =>
            typeof entry === 'string' &&
            (FRIEND_LOG_TYPES as readonly string[]).includes(entry)
    );
}
