import { useEffect, useState } from 'react';

import configRepository from '@/repositories/configRepository';

import {
    clampMutualGraphNumber,
    MUTUAL_GRAPH_LAYOUT_DEFAULTS,
    MUTUAL_GRAPH_LAYOUT_LIMITS
} from './mutualFriendsSettings';
import type {
    MutualFriendsLayoutSettingKey,
    MutualFriendsLayoutSettings
} from './mutualFriendsTypes';

interface LayoutSettingConfig {
    persist: (value: number) => void;
    decimals?: number;
}

const layoutSettingConfig: Record<
    MutualFriendsLayoutSettingKey,
    LayoutSettingConfig
> = {
    layoutIterations: {
        persist: (value) =>
            configRepository.setInt('MutualGraphLayoutIterations', value)
    },
    layoutSpacing: {
        persist: (value) =>
            configRepository.setInt('MutualGraphLayoutSpacing', value)
    },
    edgeCurvature: {
        persist: (value) =>
            configRepository.setFloat('MutualGraphEdgeCurvature', value),
        decimals: 2
    },
    communitySeparation: {
        persist: (value) =>
            configRepository.setFloat('MutualGraphCommunitySeparation', value),
        decimals: 1
    }
};

const layoutSettingKeys = Object.keys(
    layoutSettingConfig
) as MutualFriendsLayoutSettingKey[];

function normalizeLayoutSetting(
    key: MutualFriendsLayoutSettingKey,
    value: unknown
) {
    const limits = MUTUAL_GRAPH_LAYOUT_LIMITS[key];
    const nextValue = clampMutualGraphNumber(
        value,
        limits.min,
        limits.max,
        MUTUAL_GRAPH_LAYOUT_DEFAULTS[key]
    );
    const decimals = layoutSettingConfig[key].decimals;
    return Number.isInteger(decimals)
        ? Number(nextValue.toFixed(decimals))
        : nextValue;
}

export function useMutualFriendsLayoutSettings() {
    const [layoutSettings, setLayoutSettings] =
        useState<MutualFriendsLayoutSettings>(MUTUAL_GRAPH_LAYOUT_DEFAULTS);

    useEffect(() => {
        let active = true;

        Promise.all([
            configRepository.getInt(
                'MutualGraphLayoutIterations',
                MUTUAL_GRAPH_LAYOUT_DEFAULTS.layoutIterations
            ),
            configRepository.getInt(
                'MutualGraphLayoutSpacing',
                MUTUAL_GRAPH_LAYOUT_DEFAULTS.layoutSpacing
            ),
            configRepository.getFloat(
                'MutualGraphEdgeCurvature',
                MUTUAL_GRAPH_LAYOUT_DEFAULTS.edgeCurvature
            ),
            configRepository.getFloat(
                'MutualGraphCommunitySeparation',
                MUTUAL_GRAPH_LAYOUT_DEFAULTS.communitySeparation
            )
        ])
            .then(([iterations, spacing, curvature, separation]) => {
                if (!active) {
                    return;
                }

                setLayoutSettings({
                    layoutIterations: normalizeLayoutSetting(
                        'layoutIterations',
                        iterations
                    ),
                    layoutSpacing: normalizeLayoutSetting(
                        'layoutSpacing',
                        spacing
                    ),
                    edgeCurvature: normalizeLayoutSetting(
                        'edgeCurvature',
                        curvature
                    ),
                    communitySeparation: normalizeLayoutSetting(
                        'communitySeparation',
                        separation
                    )
                });
            })
            .catch(() => {});

        return () => {
            active = false;
        };
    }, []);

    function setLayoutSetting(
        key: MutualFriendsLayoutSettingKey,
        value: number
    ) {
        const nextValue = normalizeLayoutSetting(key, value);
        setLayoutSettings((current) => ({ ...current, [key]: nextValue }));
        layoutSettingConfig[key].persist(nextValue);
    }

    function resetLayoutSettings() {
        setLayoutSettings(MUTUAL_GRAPH_LAYOUT_DEFAULTS);
        for (const key of layoutSettingKeys) {
            layoutSettingConfig[key].persist(MUTUAL_GRAPH_LAYOUT_DEFAULTS[key]);
        }
    }

    return {
        layoutSettings,
        resetLayoutSettings,
        setLayoutSetting
    };
}
