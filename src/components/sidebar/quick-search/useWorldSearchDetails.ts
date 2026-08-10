import { useEffect, useState } from 'react';

import worldProfileRepository from '@/repositories/worldProfileRepository';
import type { FavoriteDetailsById } from '@/state/favoriteStoreTypes';

import { DETAIL_QUERY_MIN_LENGTH } from './quickSearchResultModel';

function indexWorldRows(
    rows: Awaited<ReturnType<typeof worldProfileRepository.searchWorlds>>
): FavoriteDetailsById {
    const detailsById: FavoriteDetailsById = {};
    for (const row of rows) {
        if (row.id) {
            detailsById[row.id] = { ...row };
        }
    }
    return detailsById;
}

export function useWorldSearchDetails(
    normalizedQuery: string
): FavoriteDetailsById {
    const [detailsById, setDetailsById] = useState<FavoriteDetailsById>({});
    const shouldLoad = normalizedQuery.length >= DETAIL_QUERY_MIN_LENGTH;

    useEffect(() => {
        if (!shouldLoad) {
            setDetailsById({});
            return;
        }

        let active = true;
        setDetailsById({});
        worldProfileRepository
            .searchWorlds(normalizedQuery)
            .then((rows) => {
                if (active) {
                    setDetailsById(indexWorldRows(rows));
                }
            })
            .catch(() => {
                if (active) {
                    setDetailsById({});
                }
            });

        return () => {
            active = false;
        };
    }, [normalizedQuery, shouldLoad]);

    return detailsById;
}
