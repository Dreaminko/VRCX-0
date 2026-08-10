import { useMemo } from 'react';

import avatarProfileRepository from '@/repositories/avatarProfileRepository';

import {
    type DetailMap,
    filterRemoteEntityCacheFallbacksById,
    getRemoteEntityCacheFallbackIds,
    loadRemoteEntityCacheFallbacksById,
    useRemoteEntityCacheFallbackLoader
} from './remoteEntityCacheFallbacks';

type AvatarDetailFallbackInput = {
    avatarIds?: unknown;
    kind: unknown;
    remoteEntityDetailsData?: DetailMap;
    remoteEntityDetailsStatus?: unknown;
};

const fetchAvatarById = (avatarId: string) =>
    avatarProfileRepository.getAvatarProfile({ avatarId });

export function getAvatarDetailFallbackIds({
    avatarIds,
    kind,
    remoteEntityDetailsData,
    remoteEntityDetailsStatus
}: AvatarDetailFallbackInput): string[] {
    return getRemoteEntityCacheFallbackIds({
        entityIds: avatarIds,
        detailSources: [remoteEntityDetailsData],
        isReady: kind === 'avatar' && remoteEntityDetailsStatus === 'ready'
    });
}

export const filterAvatarDetailFallbacksById =
    filterRemoteEntityCacheFallbacksById;

export function loadAvatarDetailFallbacksById(
    avatarIds: string[]
): Promise<DetailMap> {
    return loadRemoteEntityCacheFallbacksById(avatarIds, fetchAvatarById);
}

export function useAvatarDetailFallbacks(
    input: AvatarDetailFallbackInput
): DetailMap {
    const fallbackAvatarIds = useMemo(
        () => getAvatarDetailFallbackIds(input),
        [
            input.avatarIds,
            input.kind,
            input.remoteEntityDetailsData,
            input.remoteEntityDetailsStatus
        ]
    );

    return useRemoteEntityCacheFallbackLoader(
        fallbackAvatarIds,
        fetchAvatarById
    );
}
