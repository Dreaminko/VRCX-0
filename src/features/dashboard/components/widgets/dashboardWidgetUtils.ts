import type { DashboardConfig } from '@/features/dashboard/dashboardConfig';
import { formatDateFilter } from '@/lib/dateTime';
import { normalizeString } from '@/shared/utils/string';

export const MAX_WIDGET_ROWS = 50;

export function buildFavoriteIdSet(
    remoteFavoriteIds: readonly unknown[] | null | undefined,
    localFriendFavorites: unknown
): Set<string> {
    const ids = new Set<string>();

    for (const id of remoteFavoriteIds ?? []) {
        const normalized = normalizeString(id);
        if (normalized) {
            ids.add(normalized);
        }
    }

    const localGroups =
        localFriendFavorites &&
        typeof localFriendFavorites === 'object' &&
        !Array.isArray(localFriendFavorites)
            ? localFriendFavorites
            : {};
    for (const values of Object.values(localGroups)) {
        if (!Array.isArray(values)) {
            continue;
        }

        for (const id of values) {
            const normalized = normalizeString(id);
            if (normalized) {
                ids.add(normalized);
            }
        }
    }

    return ids;
}

export function formatWidgetTime(value: unknown) {
    if (!value) {
        return '--';
    }

    try {
        return formatDateFilter(value, 'time');
    } catch {
        return String(value);
    }
}

export function formatWidgetDate(value: unknown) {
    if (!value) {
        return '--';
    }

    try {
        return formatDateFilter(value, 'date');
    } catch {
        return String(value);
    }
}

export function getWidgetDayKey(value: unknown) {
    const date = new Date(
        typeof value === 'string' || typeof value === 'number'
            ? value
            : String(value || '')
    );
    if (Number.isNaN(date.getTime())) {
        return String(value || '').slice(0, 10);
    }

    return [
        date.getFullYear(),
        String(date.getMonth() + 1).padStart(2, '0'),
        String(date.getDate()).padStart(2, '0')
    ].join('-');
}

export function formatWidgetExactTime(value: unknown) {
    if (!value) {
        return '';
    }

    try {
        return formatDateFilter(value, 'long');
    } catch {
        return String(value);
    }
}

export function joinCompactParts(values: unknown[] = []) {
    return values.filter(Boolean).join(' • ');
}

export function isDashboardWidgetFilterActive(
    config: DashboardConfig,
    filterType: string
) {
    const filters = Array.isArray(config?.filters) ? config.filters : [];
    return filters.length === 0 || filters.includes(filterType);
}

export function getNextDashboardWidgetFilterConfig(
    config: DashboardConfig,
    filterType: string,
    filterTypes: readonly string[]
) {
    const currentFilters = Array.isArray(config.filters)
        ? config.filters.filter(
              (entry): entry is string => typeof entry === 'string'
          )
        : [];
    let filters: string[];

    if (currentFilters.length === 0) {
        filters = filterTypes.filter((entry) => entry !== filterType);
    } else if (currentFilters.includes(filterType)) {
        filters = currentFilters.filter((entry) => entry !== filterType);
        if (filters.length === 0) {
            filters = [];
        }
    } else {
        filters = [...currentFilters, filterType];
        if (filters.length === filterTypes.length) {
            filters = [];
        }
    }

    return {
        ...config,
        filters
    };
}
