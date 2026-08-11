import { useEffect, useRef, useState } from 'react';

import { usePersistedTableColumnSizing } from '@/components/data-table/dataTablePersistence';
import {
    getTablePageSizePreference,
    getTablePageSizesPreference
} from '@/services/preferencesService';
import { usePreferencesStore } from '@/state/preferencesStore';

import {
    FRIEND_LIST_COLUMN_IDS,
    FRIEND_LIST_DEFAULT_PAGE_SIZES as DEFAULT_PAGE_SIZES,
    readPersistedFriendListState as readPersistedState,
    resolveFriendListPageSize as resolvePageSize,
    sanitizeFriendListColumnOrder as sanitizeColumnOrder,
    sanitizeFriendListColumnVisibility as sanitizeColumnVisibility,
    sanitizeFriendListPageSizes as sanitizePageSizes,
    sanitizeFriendListSorting as sanitizeSorting,
    writePersistedFriendListState as writePersistedState
} from './friendListState';

export function useFriendListTableState({
    activeSearchFilterIds,
    favoritesOnly,
    filteredRowsLength,
    searchQuery
}: {
    activeSearchFilterIds: Set<string>;
    favoritesOnly: boolean;
    filteredRowsLength: number;
    searchQuery: string;
}) {
    const preferencesHydrated = usePreferencesStore(
        (state) => state.preferencesHydrated
    );
    const tablePageSizesPreference = usePreferencesStore(
        (state) => state.tablePageSizes
    );
    const [persistedState] = useState(() => readPersistedState());
    const hasWrittenSortingRef = useRef(false);
    const hasWrittenPageSizeRef = useRef(false);
    const hasWrittenTableStateRef = useRef(false);
    const [pageSizes, setPageSizes] = useState(DEFAULT_PAGE_SIZES);
    const [sorting, setSorting] = useState(() =>
        sanitizeSorting(persistedState.sorting)
    );
    const [columnVisibility, setColumnVisibility] = useState(() =>
        sanitizeColumnVisibility(persistedState.columnVisibility)
    );
    const [columnOrder, setColumnOrder] = useState(() =>
        sanitizeColumnOrder(persistedState.columnOrder)
    );
    const [columnSizing, setColumnSizing] = usePersistedTableColumnSizing({
        columnIds: FRIEND_LIST_COLUMN_IDS,
        initialValue: persistedState.columnSizing,
        writePersistedState
    });
    const [columnOrderLocked, setColumnOrderLocked] = useState(
        () => persistedState.columnOrderLocked === true
    );
    const [pagination, setPagination] = useState(() => ({
        pageIndex: 0,
        pageSize: resolvePageSize(
            persistedState.pageSize,
            DEFAULT_PAGE_SIZES,
            DEFAULT_PAGE_SIZES[1]
        )
    }));

    useEffect(() => {
        let active = true;
        Promise.all([
            getTablePageSizesPreference(DEFAULT_PAGE_SIZES),
            getTablePageSizePreference(20)
        ])
            .then(([nextPageSizes, nextPageSize]) => {
                if (!active) {
                    return;
                }
                const resolvedPageSizes = sanitizePageSizes(nextPageSizes);
                const parsedPersistedPageSize = Number.parseInt(
                    String(persistedState.pageSize ?? ''),
                    10
                );
                const hasPersistedPageSize =
                    Number.isFinite(parsedPersistedPageSize) &&
                    parsedPersistedPageSize > 0;
                const resolvedConfiguredPageSize = resolvePageSize(
                    nextPageSize,
                    resolvedPageSizes,
                    DEFAULT_PAGE_SIZES[1]
                );
                const resolvedActivePageSize = hasPersistedPageSize
                    ? resolvePageSize(
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
        const resolvedPageSizes = sanitizePageSizes(tablePageSizesPreference);
        setPageSizes(resolvedPageSizes);
        setPagination((current) => {
            const pageSize = resolvePageSize(
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
        writePersistedState({
            sorting: sanitizeSorting(sorting)
        });
    }, [sorting]);

    useEffect(() => {
        if (!hasWrittenPageSizeRef.current) {
            hasWrittenPageSizeRef.current = true;
            return;
        }
        writePersistedState({
            pageSize: pagination.pageSize
        });
    }, [pagination.pageSize]);

    useEffect(() => {
        if (!hasWrittenTableStateRef.current) {
            hasWrittenTableStateRef.current = true;
            return;
        }
        writePersistedState({
            columnVisibility: sanitizeColumnVisibility(columnVisibility),
            columnOrder: sanitizeColumnOrder(columnOrder),
            columnOrderLocked
        });
    }, [columnOrder, columnOrderLocked, columnVisibility]);

    useEffect(() => {
        setPagination((current) => ({
            ...current,
            pageIndex: 0
        }));
    }, [searchQuery, favoritesOnly, activeSearchFilterIds]);

    useEffect(() => {
        const maxPageIndex = Math.max(
            0,
            Math.ceil(filteredRowsLength / pagination.pageSize) - 1
        );
        if (pagination.pageIndex > maxPageIndex) {
            setPagination((current) => ({
                ...current,
                pageIndex: maxPageIndex
            }));
        }
    }, [filteredRowsLength, pagination.pageIndex, pagination.pageSize]);

    function resetTableLayout() {
        setColumnVisibility({});
        setColumnOrder([]);
        setColumnSizing({});
    }

    function setPageSize(value: unknown) {
        const nextPageSize = resolvePageSize(
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
        pageSizes,
        pagination,
        resetTableLayout,
        setColumnOrder,
        setColumnOrderLocked,
        setColumnSizing,
        setColumnVisibility,
        setPagination,
        setPageSize,
        setSorting,
        sorting
    };
}
