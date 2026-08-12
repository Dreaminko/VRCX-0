import type {
    ColumnOrderState,
    PaginationState,
    Updater
} from '@tanstack/react-table';
import { useEffect, useRef, useState } from 'react';

import { usePersistedTableColumnSizing } from '@/components/data-table/dataTablePersistence';
import {
    getTablePageSizePreference,
    getTablePageSizesPreference
} from '@/services/preferencesService';
import { usePreferencesStore } from '@/state/preferencesStore';

import {
    MY_AVATARS_COLUMN_IDS,
    MY_AVATARS_DEFAULT_COLUMN_VISIBILITY,
    MY_AVATARS_DEFAULT_PAGE_SIZES,
    readPersistedMyAvatarsState,
    resolveMyAvatarsColumnOrder,
    resolveMyAvatarsColumnVisibility,
    resolveMyAvatarsPageSize,
    sanitizeMyAvatarsColumnSizing,
    sanitizeMyAvatarsColumnVisibility,
    sanitizeMyAvatarsPageSizes,
    sanitizeMyAvatarsSorting,
    writePersistedMyAvatarsState
} from './myAvatarsState';
import type { MyAvatarsViewMode } from './myAvatarsTypes';

export function useMyAvatarsTableState({
    deferredSearchQuery,
    filteredCount,
    platformFilter,
    releaseStatusFilter,
    tagFilters,
    viewMode
}: {
    deferredSearchQuery: string;
    filteredCount: number;
    platformFilter: string;
    releaseStatusFilter: string;
    tagFilters: Set<string>;
    viewMode: MyAvatarsViewMode;
}) {
    const [persistedState] = useState(() => readPersistedMyAvatarsState());
    const hasWrittenSortingRef = useRef(false);
    const hasWrittenPageSizeRef = useRef(false);
    const hasWrittenTableStateRef = useRef(false);
    const preferencesHydrated = usePreferencesStore(
        (state) => state.preferencesHydrated
    );
    const tablePageSizesPreference = usePreferencesStore(
        (state) => state.tablePageSizes
    );
    const [pageSizes, setPageSizes] = useState(MY_AVATARS_DEFAULT_PAGE_SIZES);
    const [sorting, setSorting] = useState(() =>
        sanitizeMyAvatarsSorting(persistedState.sorting)
    );
    const [columnVisibility, setColumnVisibility] = useState(() =>
        resolveMyAvatarsColumnVisibility(persistedState)
    );
    const [columnOrder, setColumnOrder] = useState(() =>
        resolveMyAvatarsColumnOrder(persistedState.columnOrder)
    );
    const [columnSizing, setColumnSizing] = usePersistedTableColumnSizing({
        columnIds: MY_AVATARS_COLUMN_IDS,
        initialValue: sanitizeMyAvatarsColumnSizing(
            persistedState.columnSizing
        ),
        writePersistedState: writePersistedMyAvatarsState
    });
    const [columnOrderLocked, setColumnOrderLocked] = useState(
        () => persistedState.columnOrderLocked === true
    );
    const [pagination, setPagination] = useState<PaginationState>(() => ({
        pageIndex: 0,
        pageSize: resolveMyAvatarsPageSize(
            persistedState.pageSize,
            MY_AVATARS_DEFAULT_PAGE_SIZES,
            MY_AVATARS_DEFAULT_PAGE_SIZES[1]
        )
    }));
    useEffect(() => {
        let active = true;
        Promise.all([
            getTablePageSizesPreference(MY_AVATARS_DEFAULT_PAGE_SIZES),
            getTablePageSizePreference(20)
        ])
            .then(([nextPageSizes, nextPageSize]) => {
                if (!active) {
                    return;
                }
                const resolvedPageSizes =
                    sanitizeMyAvatarsPageSizes(nextPageSizes);
                const parsedPersistedPageSize = Number.parseInt(
                    String(persistedState.pageSize ?? ''),
                    10
                );
                const hasPersistedPageSize =
                    Number.isFinite(parsedPersistedPageSize) &&
                    parsedPersistedPageSize > 0;
                const resolvedConfiguredPageSize = resolveMyAvatarsPageSize(
                    nextPageSize,
                    resolvedPageSizes,
                    MY_AVATARS_DEFAULT_PAGE_SIZES[1]
                );
                const resolvedActivePageSize = hasPersistedPageSize
                    ? resolveMyAvatarsPageSize(
                          parsedPersistedPageSize,
                          resolvedPageSizes,
                          resolvedConfiguredPageSize
                      )
                    : resolvedConfiguredPageSize;
                setPageSizes(resolvedPageSizes);
                setPagination((current) => ({
                    ...current,
                    pageSize: resolvedActivePageSize
                }));
            })
            .catch(() => {});
        return () => {
            active = false;
        };
    }, [persistedState.pageSize]);

    useEffect(() => {
        if (!preferencesHydrated) {
            return;
        }
        const resolvedPageSizes = sanitizeMyAvatarsPageSizes(
            tablePageSizesPreference
        );
        setPageSizes(resolvedPageSizes);
        setPagination((current) => {
            const pageSize = resolveMyAvatarsPageSize(
                current.pageSize,
                resolvedPageSizes
            );
            return pageSize === current.pageSize
                ? current
                : {
                      ...current,
                      pageSize
                  };
        });
    }, [preferencesHydrated, tablePageSizesPreference]);

    useEffect(() => {
        if (!hasWrittenSortingRef.current) {
            hasWrittenSortingRef.current = true;
            return;
        }
        writePersistedMyAvatarsState({
            sorting: sanitizeMyAvatarsSorting(sorting)
        });
    }, [sorting]);

    useEffect(() => {
        if (!hasWrittenPageSizeRef.current) {
            hasWrittenPageSizeRef.current = true;
            return;
        }
        writePersistedMyAvatarsState({
            pageSize: pagination.pageSize
        });
    }, [pagination.pageSize]);

    useEffect(() => {
        if (!hasWrittenTableStateRef.current) {
            hasWrittenTableStateRef.current = true;
            return;
        }
        writePersistedMyAvatarsState({
            columnVisibility:
                sanitizeMyAvatarsColumnVisibility(columnVisibility),
            columnOrder: resolveMyAvatarsColumnOrder(columnOrder),
            columnOrderLocked
        });
    }, [columnOrder, columnOrderLocked, columnVisibility]);

    useEffect(() => {
        setPagination((current) => ({
            ...current,
            pageIndex: 0
        }));
    }, [
        deferredSearchQuery,
        platformFilter,
        releaseStatusFilter,
        tagFilters,
        viewMode
    ]);

    useEffect(() => {
        const maxPageIndex = Math.max(
            0,
            Math.ceil(filteredCount / pagination.pageSize) - 1
        );
        if (pagination.pageIndex > maxPageIndex) {
            setPagination((current) => ({
                ...current,
                pageIndex: maxPageIndex
            }));
        }
    }, [filteredCount, pagination.pageIndex, pagination.pageSize]);

    function handleColumnOrderChange(updater: Updater<ColumnOrderState>) {
        setColumnOrder((current) =>
            resolveMyAvatarsColumnOrder(
                typeof updater === 'function'
                    ? updater(resolveMyAvatarsColumnOrder(current))
                    : updater
            )
        );
    }

    function handlePageSizeChange(value: unknown) {
        const nextPageSize = resolveMyAvatarsPageSize(
            value,
            pageSizes,
            pagination.pageSize
        );
        setPagination({
            pageIndex: 0,
            pageSize: nextPageSize
        });
    }

    return {
        columnOrder,
        columnOrderLocked,
        columnSizing,
        columnVisibility,
        handleColumnOrderChange,
        handlePageSizeChange,
        initialColumnVisibility: MY_AVATARS_DEFAULT_COLUMN_VISIBILITY,
        pageSizes,
        pagination,
        setColumnOrderLocked,
        setColumnSizing,
        setColumnVisibility,
        setPagination,
        setSorting,
        sorting
    };
}
