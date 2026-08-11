import type { PaginationState } from '@tanstack/react-table';
import type { Dispatch, SetStateAction } from 'react';
import { useEffect, useRef, useState } from 'react';

import { usePersistedTableColumnSizing } from '@/components/data-table/dataTablePersistence';
import configRepository from '@/repositories/configRepository';
import { FEED_FILTER_TYPES } from '@/repositories/feedRepository';
import {
    getTablePageSizePreference,
    getTablePageSizesPreference
} from '@/services/preferencesService';
import { usePreferencesStore } from '@/state/preferencesStore';

import {
    FEED_TABLE_ORDER_COLUMN_IDS,
    FEED_TABLE_DEFAULT_PAGE_SIZES as DEFAULT_PAGE_SIZES,
    readPersistedFeedTableState as readPersistedState,
    resolveFeedPageSize as resolvePageSize,
    safeJsonParse,
    sanitizeFeedColumnOrder as sanitizeColumnOrder,
    sanitizeFeedColumnVisibility as sanitizeColumnVisibility,
    sanitizeFeedPageSizes as sanitizePageSizes,
    sanitizeFeedSorting as sanitizeSorting,
    writePersistedFeedTableState as writePersistedState
} from './feedTableState';
import type { FeedFilterType } from './feedTypes';

type UseFeedTableStateOptions = {
    activeFilters: FeedFilterType[];
    dateFrom: string;
    dateTo: string;
    deferredSearchQuery: string;
    favoritesOnly: boolean;
    scopedUserIds: readonly string[];
    setFavoritesOnly: Dispatch<SetStateAction<boolean>>;
    setFeedFilters(filters: readonly unknown[]): void;
};

export function useFeedTableState({
    activeFilters,
    dateFrom,
    dateTo,
    deferredSearchQuery,
    favoritesOnly,
    scopedUserIds,
    setFavoritesOnly,
    setFeedFilters
}: UseFeedTableStateOptions) {
    const preferencesHydrated = usePreferencesStore(
        (state) => state.preferencesHydrated
    );
    const tablePageSizesPreference = usePreferencesStore(
        (state) => state.tablePageSizes
    );
    const [persistedState] = useState(() => readPersistedState());
    const persistedPageSize = Number.parseInt(
        String(persistedState.pageSize),
        10
    );
    const hasWrittenSortingRef = useRef(false);
    const hasWrittenPageSizeRef = useRef(false);
    const hasWrittenColumnVisibilityRef = useRef(false);
    const hasWrittenTableLayoutRef = useRef(false);
    const [preferencesReady, setPreferencesReady] = useState(false);
    const [expanded, setExpanded] = useState({});
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
        columnIds: FEED_TABLE_ORDER_COLUMN_IDS,
        initialValue: persistedState.columnSizing,
        writePersistedState
    });
    const [columnOrderLocked, setColumnOrderLocked] = useState(
        () => persistedState.columnOrderLocked === true
    );
    const [pagination, setPagination] = useState<PaginationState>({
        pageIndex: 0,
        pageSize: resolvePageSize(persistedPageSize, DEFAULT_PAGE_SIZES)
    });

    useEffect(() => {
        let active = true;
        Promise.all([
            configRepository.getString('feedTableFilters', '[]'),
            configRepository.getBool('VRCX_feedTableVIPFilter', false),
            getTablePageSizesPreference(DEFAULT_PAGE_SIZES),
            getTablePageSizePreference(20)
        ])
            .then(([savedFilters, savedVip, savedPageSizes, savedPageSize]) => {
                if (!active) {
                    return;
                }
                const parsedFilters = safeJsonParse(savedFilters);
                const nextPageSizes = sanitizePageSizes(savedPageSizes);
                const resolvedSavedPageSize = resolvePageSize(
                    savedPageSize,
                    nextPageSizes
                );
                const resolvedActivePageSize = Number.isFinite(
                    persistedPageSize
                )
                    ? resolvePageSize(
                          persistedPageSize,
                          nextPageSizes,
                          resolvedSavedPageSize
                      )
                    : resolvedSavedPageSize;

                setFeedFilters(
                    Array.isArray(parsedFilters)
                        ? parsedFilters.filter((filter) =>
                              FEED_FILTER_TYPES.includes(
                                  filter as FeedFilterType
                              )
                          )
                        : []
                );
                setFavoritesOnly(Boolean(savedVip));
                setPageSizes(nextPageSizes);
                setPagination((current) => ({
                    ...current,
                    pageSize: resolvedActivePageSize
                }));
                setPreferencesReady(true);
            })
            .catch(() => {
                if (active) {
                    setPreferencesReady(true);
                }
            });
        return () => {
            active = false;
        };
    }, [persistedPageSize, setFavoritesOnly, setFeedFilters]);

    useEffect(() => {
        if (!preferencesHydrated) {
            return;
        }
        const nextPageSizes = sanitizePageSizes(tablePageSizesPreference);
        setPageSizes(nextPageSizes);
        setPagination((current) => {
            const pageSize = resolvePageSize(current.pageSize, nextPageSizes);
            return pageSize === current.pageSize
                ? current
                : {
                      ...current,
                      pageSize
                  };
        });
    }, [preferencesHydrated, tablePageSizesPreference]);

    useEffect(() => {
        if (!preferencesReady) {
            return;
        }
        configRepository.setString(
            'VRCX_feedTableFilters',
            JSON.stringify(activeFilters)
        );
    }, [activeFilters, preferencesReady]);

    useEffect(() => {
        if (!preferencesReady) {
            return;
        }
        configRepository.setBool('VRCX_feedTableVIPFilter', favoritesOnly);
    }, [favoritesOnly, preferencesReady]);

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
        if (!hasWrittenColumnVisibilityRef.current) {
            hasWrittenColumnVisibilityRef.current = true;
            return;
        }
        writePersistedState({
            columnVisibility: sanitizeColumnVisibility(columnVisibility)
        });
    }, [columnVisibility]);

    useEffect(() => {
        if (!hasWrittenTableLayoutRef.current) {
            hasWrittenTableLayoutRef.current = true;
            return;
        }
        writePersistedState({
            columnOrder: sanitizeColumnOrder(columnOrder),
            columnOrderLocked
        });
    }, [columnOrder, columnOrderLocked]);

    useEffect(() => {
        setPagination((current) => ({
            ...current,
            pageIndex: 0
        }));
    }, [
        activeFilters,
        dateFrom,
        dateTo,
        deferredSearchQuery,
        favoritesOnly,
        scopedUserIds
    ]);

    return {
        columnOrder,
        columnOrderLocked,
        columnSizing,
        columnVisibility,
        expanded,
        pageSizes,
        pagination,
        preferencesReady,
        setColumnOrder,
        setColumnOrderLocked,
        setColumnSizing,
        setColumnVisibility,
        setExpanded,
        setPagination,
        setSorting,
        sorting
    };
}
