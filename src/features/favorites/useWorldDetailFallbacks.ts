import { useMemo } from 'react';

import worldProfileRepository from '@/repositories/worldProfileRepository';

import {
    type DetailMap,
    filterRemoteEntityCacheFallbacksById,
    getRemoteEntityCacheFallbackIds,
    loadRemoteEntityCacheFallbacksById,
    useRemoteEntityCacheFallbackLoader
} from './remoteEntityCacheFallbacks';

type WorldDetailFallbackInput = {
    worldIds?: unknown;
    kind: unknown;
    localWorldDetailsById?: DetailMap;
    remoteEntityDetailsData?: DetailMap;
    remoteEntityDetailsStatus?: unknown;
};

const fetchWorldById = (worldId: string) =>
    worldProfileRepository.getWorldProfile({ worldId });

export function getWorldDetailFallbackIds({
    worldIds,
    kind,
    localWorldDetailsById,
    remoteEntityDetailsData,
    remoteEntityDetailsStatus
}: WorldDetailFallbackInput): string[] {
    return getRemoteEntityCacheFallbackIds({
        entityIds: worldIds,
        detailSources: [remoteEntityDetailsData, localWorldDetailsById],
        isReady: kind === 'world' && remoteEntityDetailsStatus === 'ready'
    });
}

export const filterWorldDetailFallbacksById =
    filterRemoteEntityCacheFallbacksById;

export function loadWorldDetailFallbacksById(
    worldIds: string[]
): Promise<DetailMap> {
    return loadRemoteEntityCacheFallbacksById(worldIds, fetchWorldById);
}

export function useWorldDetailFallbacks(
    input: WorldDetailFallbackInput
): DetailMap {
    const fallbackWorldIds = useMemo(
        () => getWorldDetailFallbackIds(input),
        [
            input.worldIds,
            input.kind,
            input.localWorldDetailsById,
            input.remoteEntityDetailsData,
            input.remoteEntityDetailsStatus
        ]
    );

    return useRemoteEntityCacheFallbackLoader(fallbackWorldIds, fetchWorldById);
}
