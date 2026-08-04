import { useEffect, useMemo } from 'react';

import {
    getVisibleKnownSizeRows,
    positionKnownSizeRows
} from '@/lib/knownSizeVirtualRows';
import { useScrollViewportMetrics } from '@/lib/useScrollViewportMetrics';

import {
    getFavoritesCardHeight,
    type FavoritesDensityConfig
} from './favoritesDensity';
import type { FavoriteItem } from './favoritesTypes';

const FAVORITES_GRID_HORIZONTAL_INSET = 8;
const FAVORITES_GRID_CARD_PADDING = 2;
const FAVORITES_GRID_OVERSCAN_MIN = 420;

type FavoritesGridRowInput = {
    key: string;
    height: number;
    cellHeight: number;
    items: FavoriteItem[];
};

function buildFavoritesGridRows({
    cellHeight,
    gridColumnCount,
    gridGap,
    items
}: {
    cellHeight: number;
    gridColumnCount: number;
    gridGap: number;
    items: readonly FavoriteItem[];
}) {
    const safeItems = Array.isArray(items) ? items : [];
    const rows: FavoritesGridRowInput[] = [];

    for (let index = 0; index < safeItems.length; index += gridColumnCount) {
        const isLastRow = index + gridColumnCount >= safeItems.length;
        rows.push({
            key: `favorites-grid-row:${index}`,
            height: cellHeight + (isLastRow ? 0 : gridGap),
            cellHeight,
            items: safeItems.slice(index, index + gridColumnCount)
        });
    }

    return positionKnownSizeRows(rows);
}

type UseFavoritesVirtualGridOptions = {
    densityConfig: FavoritesDensityConfig;
    items: readonly FavoriteItem[];
    resetKey: string;
    showGroupLabel?: boolean;
};

export function useFavoritesVirtualGrid({
    densityConfig,
    items,
    resetKey,
    showGroupLabel
}: UseFavoritesVirtualGridOptions) {
    const { resetScrollTop, viewportMetrics, viewportRef } =
        useScrollViewportMetrics();

    useEffect(() => {
        resetScrollTop();
    }, [resetKey, resetScrollTop]);

    const gridPadding = FAVORITES_GRID_CARD_PADDING;
    const gridInset = gridPadding * 2;
    const gridGap = Math.max(0, densityConfig.gridGap - gridInset);
    const gridMinWidth = densityConfig.gridMinWidth + gridInset;
    const safeWidth = Math.max(
        0,
        (Number(viewportMetrics.width) || 0) - FAVORITES_GRID_HORIZONTAL_INSET
    );
    const gridColumnCount = Math.max(
        1,
        Math.floor((safeWidth + gridGap) / (gridMinWidth + gridGap)) || 1
    );
    const gridColumnWidth =
        (safeWidth - gridGap * (gridColumnCount - 1)) / gridColumnCount;
    const cardWidth = Math.max(0, gridColumnWidth - gridInset);
    const cardHeight = getFavoritesCardHeight({
        config: densityConfig,
        columnWidth: cardWidth,
        showGroupLabel
    });
    const cellHeight = cardHeight + gridInset;

    const positionedRows = useMemo(
        () =>
            buildFavoritesGridRows({
                cellHeight,
                gridColumnCount,
                gridGap,
                items
            }),
        [cellHeight, gridColumnCount, gridGap, items]
    );

    const visibleRows = useMemo(() => {
        const overscan = Math.max(
            FAVORITES_GRID_OVERSCAN_MIN,
            viewportMetrics.viewportHeight
        );
        return getVisibleKnownSizeRows({
            rows: positionedRows.rows,
            scrollTop: viewportMetrics.scrollTop,
            viewportHeight: viewportMetrics.viewportHeight,
            overscan
        });
    }, [
        positionedRows.rows,
        viewportMetrics.scrollTop,
        viewportMetrics.viewportHeight
    ]);

    return {
        cardHeight,
        gridColumnCount,
        gridGap,
        gridMinWidth,
        gridPadding,
        totalHeight: positionedRows.totalHeight,
        viewportRef,
        visibleRows
    };
}
