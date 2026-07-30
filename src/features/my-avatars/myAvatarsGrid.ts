import {
    getVisibleKnownSizeRows,
    positionKnownSizeRows
} from '@/lib/knownSizeVirtualRows';

import {
    MY_AVATARS_DEFAULT_GRID_DENSITY,
    sanitizeMyAvatarsGridDensity
} from './myAvatarsState';
import type {
    MyAvatarRow,
    MyAvatarsGridDensity,
    MyAvatarsGridDensityConfig,
    MyAvatarsGridRow
} from './myAvatarsTypes';

const MY_AVATARS_GRID_DENSITY_CONFIGS = Object.freeze({
    standard: Object.freeze({
        value: 'standard',
        gridGap: 8,
        gridMinWidth: 180,
        imageHeightRatio: 0.66,
        overlayPaddingX: 8,
        overlayPaddingY: 7,
        overlayPaddingTop: 24,
        overlayNameOnlyPaddingTop: 16,
        overlayGap: 4,
        nameFontSize: 13,
        nameLineHeight: 1.15,
        tagFontSize: 9,
        maxVisibleTags: 2,
        rowPaddingY: 4
    }),
    compact: Object.freeze({
        value: 'compact',
        gridGap: 7,
        gridMinWidth: 150,
        imageHeightRatio: 0.64,
        overlayPaddingX: 7,
        overlayPaddingY: 6,
        overlayPaddingTop: 22,
        overlayNameOnlyPaddingTop: 14,
        overlayGap: 4,
        nameFontSize: 12,
        nameLineHeight: 1.12,
        tagFontSize: 8,
        maxVisibleTags: 1,
        rowPaddingY: 3
    }),
    dense: Object.freeze({
        value: 'dense',
        gridGap: 6,
        gridMinWidth: 125,
        imageHeightRatio: 0.6,
        overlayPaddingX: 6,
        overlayPaddingY: 5,
        overlayPaddingTop: 18,
        overlayNameOnlyPaddingTop: 12,
        overlayGap: 3,
        nameFontSize: 11,
        nameLineHeight: 1.1,
        tagFontSize: 8,
        maxVisibleTags: 0,
        rowPaddingY: 3
    })
});

type MyAvatarsGridDensityKey = keyof typeof MY_AVATARS_GRID_DENSITY_CONFIGS;

export function getMyAvatarsGridDensityConfig(
    value: unknown
): MyAvatarsGridDensityConfig {
    const densityKey = sanitizeMyAvatarsGridDensity(
        value
    ) as MyAvatarsGridDensityKey;
    return (
        MY_AVATARS_GRID_DENSITY_CONFIGS[densityKey] ||
        MY_AVATARS_GRID_DENSITY_CONFIGS[MY_AVATARS_DEFAULT_GRID_DENSITY]
    );
}

type MyAvatarsGridMetricsInput = {
    gridDensity: MyAvatarsGridDensity;
    width: number;
};

export function getMyAvatarsGridMetrics({
    gridDensity,
    width
}: MyAvatarsGridMetricsInput) {
    const densityConfig = getMyAvatarsGridDensityConfig(gridDensity);
    const gridGap = densityConfig.gridGap;
    const gridMinWidth = densityConfig.gridMinWidth;
    const gridColumnCount = Math.max(
        1,
        Math.floor((width + gridGap) / (gridMinWidth + gridGap)) || 1
    );
    const gridColumnWidth =
        width > 0
            ? Math.max(
                  gridMinWidth,
                  (width - gridGap * Math.max(0, gridColumnCount - 1)) /
                      gridColumnCount
              )
            : gridMinWidth;
    const gridRowHeight = Math.ceil(
        gridColumnWidth * densityConfig.imageHeightRatio +
            densityConfig.rowPaddingY +
            gridGap
    );

    return {
        densityConfig,
        gridGap,
        gridMinWidth,
        gridColumnCount,
        gridColumnWidth,
        gridRowHeight
    };
}

type BuildMyAvatarsGridRowsInput = {
    avatars: readonly MyAvatarRow[] | null | undefined;
    gridColumnCount: number;
    gridRowHeight: number;
};

export function buildMyAvatarsGridRows({
    avatars,
    gridColumnCount,
    gridRowHeight
}: BuildMyAvatarsGridRowsInput): MyAvatarsGridRow[] {
    const rows: Omit<MyAvatarsGridRow, 'top'>[] = [];
    const visibleAvatars = Array.isArray(avatars) ? avatars : [];
    for (
        let index = 0;
        index < visibleAvatars.length;
        index += gridColumnCount
    ) {
        rows.push({
            key: `grid-row:${index}`,
            avatars: visibleAvatars.slice(index, index + gridColumnCount),
            height: gridRowHeight
        });
    }
    return positionKnownSizeRows(rows).rows;
}

type VisibleMyAvatarsGridRowsInput = {
    gridRows: readonly MyAvatarsGridRow[] | null | undefined;
    scrollTop: unknown;
    viewportHeight: unknown;
};

export function getVisibleMyAvatarsGridRows({
    gridRows,
    scrollTop,
    viewportHeight
}: VisibleMyAvatarsGridRowsInput) {
    const overscan = Math.max(480, Number(viewportHeight) || 0);
    return getVisibleKnownSizeRows({
        rows: gridRows,
        scrollTop,
        viewportHeight,
        overscan
    });
}
