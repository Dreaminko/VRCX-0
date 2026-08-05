import { useCallback, useEffect, useState } from 'react';

import type { QuickSearchResult } from '../quickSearchCatalog';
import {
    loadQuickSearchHistory,
    promoteQuickSearchHistoryEntry,
    recordQuickSearchHistory,
    type QuickSearchHistoryScope
} from './quickSearchHistory';

export function useQuickSearchHistory({
    currentEndpoint,
    currentUserId,
    open
}: {
    currentEndpoint?: string | null;
    currentUserId?: string | null;
    open: boolean;
}) {
    const scopeKey =
        currentEndpoint && currentUserId
            ? JSON.stringify([currentEndpoint.trim(), currentUserId.trim()])
            : null;
    const [history, setHistory] = useState<{
        scopeKey: string | null;
        items: QuickSearchResult[];
    }>({ scopeKey: null, items: [] });

    useEffect(() => {
        let active = true;
        if (!open || !scopeKey || !currentEndpoint || !currentUserId) {
            return () => {
                active = false;
            };
        }
        const scope: QuickSearchHistoryScope = {
            endpoint: currentEndpoint,
            userId: currentUserId
        };
        void loadQuickSearchHistory(scope).then((items) => {
            if (active) {
                setHistory({ scopeKey, items });
            }
        });
        return () => {
            active = false;
        };
    }, [currentEndpoint, currentUserId, open, scopeKey]);

    const remember = useCallback(
        (item: QuickSearchResult) => {
            if (!scopeKey || !currentEndpoint || !currentUserId) {
                return;
            }
            setHistory((current) => ({
                scopeKey,
                items: promoteQuickSearchHistoryEntry(
                    current.scopeKey === scopeKey ? current.items : [],
                    item
                ).map((entry) => ({
                    ...entry,
                    source: 'history'
                }))
            }));
            void recordQuickSearchHistory(
                {
                    endpoint: currentEndpoint,
                    userId: currentUserId
                },
                item
            ).catch(() => undefined);
        },
        [currentEndpoint, currentUserId, scopeKey]
    );

    return {
        items: history.scopeKey === scopeKey ? history.items : [],
        remember
    };
}
