import {
    useCallback,
    useDeferredValue,
    useEffect,
    useMemo,
    useState
} from 'react';

import { useTodayDate } from '@/lib/useTodayDate';
import {
    FEED_FILTER_TYPES,
    type FeedFilterType
} from '@/repositories/feedRepository';

import { parseDateInput, toDateInputValue } from './feedRows';
import type { FeedDateRange } from './feedTypes';

const EMPTY_SCOPED_USER_IDS: readonly string[] = [];

function normalizeScopedUserIds(userIds: readonly string[]): string[] {
    return [...new Set(userIds.map((userId) => userId.trim()).filter(Boolean))];
}

function normalizeFeedFilters(filters: readonly unknown[]): FeedFilterType[] {
    const nextFilters = (Array.isArray(filters) ? filters : []).filter(
        (filter): filter is FeedFilterType =>
            typeof filter === 'string' &&
            FEED_FILTER_TYPES.includes(filter as FeedFilterType)
    );
    return [...new Set(nextFilters)];
}

export function useFeedFilters({
    routeScopedUserIds = EMPTY_SCOPED_USER_IDS
}: {
    routeScopedUserIds?: readonly string[];
} = {}) {
    const [searchDraft, setSearchDraft] = useState('');
    const [searchQuery, setSearchQuery] = useState('');
    const [dateFrom, setDateFrom] = useState('');
    const [dateTo, setDateTo] = useState('');
    const [dateDraftFrom, setDateDraftFrom] = useState('');
    const [dateDraftTo, setDateDraftTo] = useState('');
    const [dateFilterOpen, setDateFilterOpen] = useState(false);
    const [activeFilters, setActiveFilters] = useState<FeedFilterType[]>([]);
    const [favoritesOnly, setFavoritesOnly] = useState(false);
    const normalizedRouteScopedUserIds = useMemo(
        () => normalizeScopedUserIds(routeScopedUserIds),
        [routeScopedUserIds]
    );
    const [scopedUserIds, setScopedUserIds] = useState<string[]>(
        normalizedRouteScopedUserIds
    );
    const deferredSearchQuery = useDeferredValue(searchQuery);
    const deferredScopedUserIds = useDeferredValue(scopedUserIds);
    const todayDate = useTodayDate();

    const setUserScope = useCallback((nextUserIds: readonly string[]) => {
        const normalized = normalizeScopedUserIds(nextUserIds);
        setScopedUserIds((current) =>
            current.length === normalized.length &&
            current.every((userId, index) => userId === normalized[index])
                ? current
                : normalized
        );
        if (normalized.length) {
            setFavoritesOnly(false);
        }
    }, []);

    useEffect(() => {
        setUserScope(normalizedRouteScopedUserIds);
    }, [normalizedRouteScopedUserIds, setUserScope]);

    const setFeedFilters = useCallback((nextFilters: readonly unknown[]) => {
        const nextUniqueFilters = normalizeFeedFilters(nextFilters);
        setActiveFilters(
            nextUniqueFilters.length === FEED_FILTER_TYPES.length
                ? []
                : nextUniqueFilters
        );
    }, []);

    const toggleFeedFilter = useCallback((filter: FeedFilterType) => {
        setActiveFilters((current) => {
            const nextFilters = current.includes(filter)
                ? current.filter((entry) => entry !== filter)
                : [...current, filter];
            return nextFilters.length === FEED_FILTER_TYPES.length
                ? []
                : nextFilters;
        });
    }, []);

    const commitSearch = useCallback(
        (nextValue: string = searchDraft) => {
            setSearchQuery(nextValue);
        },
        [searchDraft]
    );

    const clearSearch = useCallback(() => {
        setSearchDraft('');
        setSearchQuery('');
    }, []);

    const applyDateFilter = useCallback(() => {
        if (dateDraftFrom && dateDraftTo && dateDraftFrom > dateDraftTo) {
            setDateFrom(dateDraftTo);
            setDateTo(dateDraftFrom);
        } else {
            setDateFrom(dateDraftFrom);
            setDateTo(dateDraftTo);
        }
        setDateFilterOpen(false);
    }, [dateDraftFrom, dateDraftTo]);

    const clearDateFilter = useCallback(() => {
        setDateDraftFrom('');
        setDateDraftTo('');
        setDateFrom('');
        setDateTo('');
        setDateFilterOpen(false);
    }, []);

    const dateDraftRange = useMemo(() => {
        const from = parseDateInput(dateDraftFrom);
        const to = parseDateInput(dateDraftTo);
        return from || to ? { from, to } : undefined;
    }, [dateDraftFrom, dateDraftTo]);

    useEffect(() => {
        if (!dateFilterOpen) {
            return;
        }
        setDateDraftFrom(dateFrom);
        setDateDraftTo(dateTo);
    }, [dateFilterOpen, dateFrom, dateTo]);

    const onDateRangeSelect = useCallback((range?: FeedDateRange) => {
        setDateDraftFrom(toDateInputValue(range?.from));
        setDateDraftTo(toDateInputValue(range?.to));
    }, []);

    return {
        activeFilters,
        deferredScopedUserIds,
        scopedUserIds,
        setUserScope,
        dateDraftFrom,
        dateDraftRange,
        dateDraftTo,
        dateFilterOpen,
        dateFrom,
        dateTo,
        deferredSearchQuery,
        favoritesOnly,
        feedFilterTypes: FEED_FILTER_TYPES,
        searchDraft,
        todayDate,
        applyDateFilter,
        clearDateFilter,
        clearSearch,
        commitSearch,
        onDateRangeSelect,
        setDateFilterOpen,
        setFavoritesOnly,
        setFeedFilters,
        setSearchDraft,
        toggleFeedFilter
    };
}
