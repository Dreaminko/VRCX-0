import type {
    PaginationState,
    RowData,
    SortingState
} from '@tanstack/react-table';
import { useEffect, useMemo, useRef, useState } from 'react';

import type { AppColumnDef } from '@/components/data-table/appTable';
import { useAppTable } from '@/components/data-table/appTable';
import {
    sanitizeTableColumnOrder,
    sanitizeTableColumnVisibility,
    usePersistedDataTableLayout
} from '@/components/data-table/dataTablePersistence';
import { usePreferencesStore } from '@/state/preferencesStore';

const GROUP_MODERATION_DEFAULT_PAGE_SIZE = 25;
const GROUP_MODERATION_DEFAULT_PAGE_SIZES = [10, 25, 50, 100];

function sanitizeSorting(
    value: unknown,
    columnIds: readonly string[]
): SortingState {
    if (!Array.isArray(value)) {
        return [];
    }
    return value.reduce<SortingState>((result, entry) => {
        const id = (entry as { id?: unknown })?.id;
        const desc = (entry as { desc?: unknown })?.desc;
        if (
            typeof id === 'string' &&
            typeof desc === 'boolean' &&
            columnIds.includes(id)
        ) {
            result.push({ id, desc });
        }
        return result;
    }, []);
}

export function useGroupModerationTable<TData extends RowData>({
    columnIds,
    columns,
    paged,
    rows,
    tableId
}: {
    columnIds: string[];
    columns: AppColumnDef<TData>[];
    paged: boolean;
    rows: TData[];
    tableId: string;
}) {
    const {
        columnOrder,
        columnOrderLocked,
        columnSizing,
        columnVisibility,
        persistedState,
        setColumnOrder,
        setColumnOrderLocked,
        setColumnSizing,
        setColumnVisibility,
        writePersistedState
    } = usePersistedDataTableLayout({ tableId, columnIds });
    const preferencesHydrated = usePreferencesStore(
        (state) => state.preferencesHydrated
    );
    const preferredPageSize = usePreferencesStore(
        (state) => state.tablePageSize
    );
    const preferredPageSizes = usePreferencesStore(
        (state) => state.tablePageSizes
    );
    const hasWrittenLayoutRef = useRef(false);
    const appliedPreferredPageSizeRef = useRef(false);
    const [sorting, setSorting] = useState<SortingState>(() =>
        sanitizeSorting(persistedState.sorting, columnIds)
    );
    const [pagination, setPagination] = useState<PaginationState>({
        pageIndex: 0,
        pageSize: GROUP_MODERATION_DEFAULT_PAGE_SIZE
    });

    const pageSizes = useMemo(() => {
        const options = Array.isArray(preferredPageSizes)
            ? preferredPageSizes.filter(
                  (value): value is number =>
                      typeof value === 'number' &&
                      Number.isFinite(value) &&
                      value > 0
              )
            : [];
        return options.length ? options : GROUP_MODERATION_DEFAULT_PAGE_SIZES;
    }, [preferredPageSizes]);

    useEffect(() => {
        if (
            !paged ||
            appliedPreferredPageSizeRef.current ||
            !preferencesHydrated
        ) {
            return;
        }
        appliedPreferredPageSizeRef.current = true;
        if (preferredPageSize > 0) {
            setPagination((current) =>
                current.pageSize === preferredPageSize
                    ? current
                    : { ...current, pageSize: preferredPageSize }
            );
        }
    }, [paged, preferencesHydrated, preferredPageSize]);

    useEffect(() => {
        if (!hasWrittenLayoutRef.current) {
            hasWrittenLayoutRef.current = true;
            return;
        }
        writePersistedState({
            columnOrder: sanitizeTableColumnOrder(
                columnOrder,
                columnIds
            ),
            columnVisibility: sanitizeTableColumnVisibility(
                columnVisibility,
                columnIds
            ),
            sorting: sanitizeSorting(sorting, columnIds)
        });
    }, [
        columnIds,
        columnOrder,
        columnVisibility,
        sorting,
        writePersistedState
    ]);

    useEffect(() => {
        if (!paged) {
            return;
        }
        const maxPageIndex = Math.max(
            0,
            Math.ceil(rows.length / pagination.pageSize) - 1
        );
        if (pagination.pageIndex > maxPageIndex) {
            setPagination((current) => ({
                ...current,
                pageIndex: maxPageIndex
            }));
        }
    }, [paged, pagination.pageIndex, pagination.pageSize, rows.length]);

    const table = useAppTable<TData>({
        columns,
        data: rows,
        state: {
            columnOrder,
            columnSizing,
            columnVisibility,
            ...(paged ? { sorting, pagination } : {})
        },
        onColumnOrderChange: setColumnOrder,
        onColumnSizingChange: setColumnSizing,
        onColumnVisibilityChange: setColumnVisibility,
        onSortingChange: paged ? setSorting : undefined,
        onPaginationChange: paged ? setPagination : undefined,
        manualSorting: !paged,
        manualPagination: !paged,
        enableColumnResizing: true,
        columnResizeMode: 'onChange',
        meta: {
            columnOrderLocked,
            setColumnOrderLocked
        }
    });

    return { pageSizes, pagination, setPagination, table };
}
