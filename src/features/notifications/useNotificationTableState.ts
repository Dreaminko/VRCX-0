import { useEffect, useRef, useState } from 'react';

import { usePreferencesStore } from '@/state/preferencesStore';

import {
    NOTIFICATION_TABLE_DEFAULT_PAGE_SIZES as DEFAULT_PAGE_SIZES,
    readPersistedNotificationTableState as readPersistedState,
    resolveNotificationPageSize as resolvePageSize,
    sanitizeNotificationPageSizes as sanitizePageSizes,
    writePersistedNotificationTableState as writePersistedState
} from './notificationTableState';

type NotificationPagination = {
    pageIndex: number;
    pageSize: number;
};

export function useNotificationTableState({
    activeTypes,
    deferredSearchQuery,
    quickFilter
}: {
    activeTypes: string[];
    deferredSearchQuery: string;
    quickFilter: string;
}) {
    const preferencesHydrated = usePreferencesStore(
        (state) => state.preferencesHydrated
    );
    const tablePageSizePreference = usePreferencesStore(
        (state) => state.tablePageSize
    );
    const tablePageSizesPreference = usePreferencesStore(
        (state) => state.tablePageSizes
    );

    const [persistedState] = useState(() => readPersistedState());
    const persistedPageSize = Number.parseInt(
        String(persistedState.pageSize ?? ''),
        10
    );
    const hasPersistedPageSize =
        Number.isFinite(persistedPageSize) && persistedPageSize > 0;
    const hasStoredPageSizeRef = useRef(hasPersistedPageSize);
    const storedPageSizeRef = useRef(
        hasPersistedPageSize ? persistedPageSize : null
    );
    const hasWrittenPageSizeRef = useRef(false);
    const [pageSizes, setPageSizes] = useState(DEFAULT_PAGE_SIZES);
    const [pagination, setPagination] = useState<NotificationPagination>({
        pageIndex: 0,
        pageSize: resolvePageSize(persistedPageSize)
    });

    useEffect(() => {
        if (!hasWrittenPageSizeRef.current) {
            hasWrittenPageSizeRef.current = true;
            return;
        }
        hasStoredPageSizeRef.current = true;
        storedPageSizeRef.current = pagination.pageSize;
        writePersistedState({
            pageSize: pagination.pageSize
        });
    }, [pagination.pageSize]);

    useEffect(() => {
        if (!preferencesHydrated) {
            return;
        }
        const resolvedPageSizes = sanitizePageSizes(tablePageSizesPreference);
        const configuredPageSize = resolvePageSize(
            tablePageSizePreference,
            resolvedPageSizes
        );
        setPageSizes(resolvedPageSizes);
        setPagination((current) => {
            const storedPageSize = Number.isFinite(storedPageSizeRef.current)
                ? storedPageSizeRef.current
                : current.pageSize;
            const activePageSize = hasStoredPageSizeRef.current
                ? resolvePageSize(
                      storedPageSize,
                      resolvedPageSizes,
                      configuredPageSize
                  )
                : configuredPageSize;
            storedPageSizeRef.current = activePageSize;
            return activePageSize === current.pageSize
                ? current
                : {
                      ...current,
                      pageSize: activePageSize
                  };
        });
    }, [
        preferencesHydrated,
        tablePageSizePreference,
        tablePageSizesPreference
    ]);

    useEffect(() => {
        setPagination((current) => ({
            ...current,
            pageIndex: 0
        }));
    }, [activeTypes, deferredSearchQuery, quickFilter]);

    function handlePageSizeChange(value: unknown) {
        setPagination({
            pageIndex: 0,
            pageSize: resolvePageSize(value, pageSizes, pagination.pageSize)
        });
    }

    return {
        handlePageSizeChange,
        pageSizes,
        pagination,
        setPagination
    };
}
