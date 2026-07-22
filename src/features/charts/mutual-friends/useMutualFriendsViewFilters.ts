import { useCallback, useState } from 'react';

import {
    MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS,
    MUTUAL_GRAPH_MIN_DEGREE_LIMITS
} from './mutualFriendsFilters';
import { clampMutualGraphNumber } from './mutualFriendsSettings';
import type { MutualFriendsViewFilters } from './mutualFriendsTypes';

export function useMutualFriendsViewFilters() {
    const [filters, setFilters] = useState<MutualFriendsViewFilters>(
        MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS
    );

    const setSearchQuery = useCallback((searchQuery: string) => {
        setFilters((current) => ({ ...current, searchQuery }));
    }, []);

    const setMinDegree = useCallback((minDegree: number) => {
        setFilters((current) => ({
            ...current,
            minDegree: clampMutualGraphNumber(
                minDegree,
                MUTUAL_GRAPH_MIN_DEGREE_LIMITS.min,
                MUTUAL_GRAPH_MIN_DEGREE_LIMITS.max,
                MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS.minDegree
            )
        }));
    }, []);

    const toggleFocusedCommunity = useCallback((communityIndex: number) => {
        setFilters((current) => ({
            ...current,
            focusedCommunity:
                current.focusedCommunity === communityIndex
                    ? null
                    : communityIndex
        }));
    }, []);

    const clearFilters = useCallback(() => {
        setFilters(MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS);
    }, []);

    return {
        filters,
        setSearchQuery,
        setMinDegree,
        toggleFocusedCommunity,
        clearFilters
    };
}
