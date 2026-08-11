import {
    createdTime,
    rowMatchesSearch,
    sortPreviousInstanceRows
} from '@/components/dialogs/previous-instances-table/previousInstancesRows';

import type { PreviousInstanceRow } from './instance-activity/instanceActivityTypes';

export type PreviousInstanceSortKey = 'date' | 'location' | 'duration';
export type InstanceHistoryMode = 'search' | 'day';

export function buildInstanceHistorySearchParams({
    currentUserId,
    mode,
    userId
}: {
    currentUserId: string;
    mode: InstanceHistoryMode;
    userId: string;
}) {
    const params = new URLSearchParams();
    if (mode === 'day') {
        params.set('mode', 'day');
    }
    if (userId && userId !== currentUserId) {
        params.set('scope', 'user');
        params.set('id', userId);
    }
    return params;
}

function dateRangeContains(
    row: PreviousInstanceRow,
    from: Date | null,
    to: Date | null
) {
    if (!from && !to) {
        return true;
    }
    const value = createdTime(row);
    if (!value) {
        return false;
    }
    if (from && value < from.getTime()) {
        return false;
    }
    if (to && value > to.getTime()) {
        return false;
    }
    return true;
}

export function filterAndSortInstanceHistoryRows({
    rows,
    query,
    from,
    to,
    sortKey,
    sortDesc
}: {
    rows: PreviousInstanceRow[];
    query: string;
    from: Date | null;
    to: Date | null;
    sortKey: PreviousInstanceSortKey;
    sortDesc: boolean;
}) {
    const normalizedQuery = query.trim();
    const dateRows = rows.filter((row) => dateRangeContains(row, from, to));
    const filteredRows = normalizedQuery
        ? dateRows.filter((row) => rowMatchesSearch(row, normalizedQuery))
        : dateRows;
    return sortPreviousInstanceRows(filteredRows, sortKey, sortDesc);
}
