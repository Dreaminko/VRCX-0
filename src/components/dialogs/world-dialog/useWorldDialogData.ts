import {
    useEffect,
    useEffectEvent,
    useState,
    type MutableRefObject
} from 'react';
import { useTranslation } from 'react-i18next';

import type { EntityRecord } from '@/domain/entities/profileEntities';
import { getFileAnalysisForUnityPackages } from '@/lib/fileAnalysis';
import { readWorldCacheInfo } from '@/lib/worldAssetBundle';
import gameLogRepository from '@/repositories/gameLogRepository';
import groupProfileRepository from '@/repositories/groupProfileRepository';
import memoPersistenceRepository from '@/repositories/memoPersistenceRepository';
import worldProfileRepository from '@/repositories/worldProfileRepository';
import { persistFavoriteWorldDetails } from '@/services/favoriteWorldCacheService';
import { useVrchatConfigStore } from '@/state/vrchatConfigStore';

import {
    defaultWorldSideData,
    groupOptionId,
    worldLoadErrorDescription
} from './worldDialogHelpers';
import { normalizeEntityId } from './worldInstances';

type WorldDialogNewInstanceGroups = Awaited<
    ReturnType<typeof groupProfileRepository.getUserGroups>
>;

export type WorldPreviousInstances = Awaited<
    ReturnType<typeof gameLogRepository.getPreviousInstancesByWorldId>
>;

export type WorldWorldSideData = {
    cache: Awaited<ReturnType<typeof readWorldCacheInfo>>;
    fileAnalysis: Awaited<ReturnType<typeof getFileAnalysisForUnityPackages>>;
};

interface UseWorldDialogDataInput {
    normalizedWorldId: string;
    profileWorldId: string;
    seedData: EntityRecord | null;
    currentEndpoint: string;
    currentUserId: string | null;
    isCurrentWorldTarget: (worldId: string, endpoint: string) => boolean;
    memoRevisionRef: MutableRefObject<number>;
}

export function useWorldDialogData({
    normalizedWorldId,
    profileWorldId,
    seedData,
    currentEndpoint,
    currentUserId,
    isCurrentWorldTarget,
    memoRevisionRef
}: UseWorldDialogDataInput) {
    const { t } = useTranslation();
    const sdkUnityVersion = useVrchatConfigStore((state) =>
        String(state.snapshot?.sdkUnityVersion || '')
    );
    const [world, setWorld] = useState(() =>
        seedData ? worldProfileRepository.normalize(seedData) : null
    );
    const [loadStatus, setLoadStatus] = useState(
        normalizedWorldId ? 'running' : 'idle'
    );
    const [detail, setDetail] = useState('');
    const [memo, setMemo] = useState('');
    const [previousInstances, setPreviousInstances] =
        useState<WorldPreviousInstances>([]);
    const [hasPersistData, setHasPersistData] = useState(false);
    const [worldSideData, setWorldSideData] = useState<WorldWorldSideData>(() =>
        defaultWorldSideData()
    );
    const [newInstanceGroups, setNewInstanceGroups] =
        useState<WorldDialogNewInstanceGroups>([]);
    const worldAssetUrl =
        typeof world?.assetUrl === 'string' ? world.assetUrl : undefined;
    const worldId = world?.id;
    const worldUnityPackages = world?.unityPackages;
    const currentWorldTargetMatches = useEffectEvent(isCurrentWorldTarget);
    const translateWorldDetail = useEffectEvent((key: string) => t(key));
    const describeWorldLoadError = useEffectEvent(
        (error: unknown, worldId: string, key: string) =>
            worldLoadErrorDescription(error, t, worldId, key)
    );

    useEffect(() => {
        setWorld(seedData ? worldProfileRepository.normalize(seedData) : null);
    }, [seedData]);

    useEffect(() => {
        setWorldSideData(defaultWorldSideData());
    }, [profileWorldId]);

    useEffect(() => {
        let active = true;

        if (!currentUserId) {
            setNewInstanceGroups([]);
            return () => {
                active = false;
            };
        }

        groupProfileRepository
            .getUserGroups({
                userId: currentUserId
            })
            .then((groups) => {
                if (!active) {
                    return;
                }
                setNewInstanceGroups(
                    (Array.isArray(groups) ? groups : [])
                        .filter((group) => groupOptionId(group))
                        .sort((left, right) =>
                            normalizeEntityId(left?.name).localeCompare(
                                normalizeEntityId(right?.name)
                            )
                        )
                );
            })
            .catch(() => {
                if (active) {
                    setNewInstanceGroups([]);
                }
            });

        return () => {
            active = false;
        };
    }, [currentEndpoint, currentUserId]);

    useEffect(() => {
        let active = true;

        if (!worldId) {
            setWorldSideData(defaultWorldSideData());
            return () => {
                active = false;
            };
        }

        const targetWorldId = worldId;
        const targetEndpoint = currentEndpoint;
        Promise.allSettled([
            readWorldCacheInfo(
                {
                    id: worldId,
                    assetUrl: worldAssetUrl,
                    unityPackages: worldUnityPackages
                },
                sdkUnityVersion
            ),
            getFileAnalysisForUnityPackages({
                unityPackages: worldUnityPackages,
                sdkUnityVersion,
                endpoint: targetEndpoint
            })
        ])
            .then(([cacheResult, fileAnalysisResult]) => {
                if (
                    active &&
                    currentWorldTargetMatches(targetWorldId, targetEndpoint)
                ) {
                    setWorldSideData({
                        cache:
                            cacheResult.status === 'fulfilled'
                                ? cacheResult.value
                                : defaultWorldSideData().cache,
                        fileAnalysis:
                            fileAnalysisResult.status === 'fulfilled'
                                ? fileAnalysisResult.value
                                : {}
                    });
                }
            })
            .catch(() => {
                if (
                    active &&
                    currentWorldTargetMatches(targetWorldId, targetEndpoint)
                ) {
                    setWorldSideData(defaultWorldSideData());
                }
            });

        return () => {
            active = false;
        };
    }, [
        currentEndpoint,
        sdkUnityVersion,
        world?.updatedAt,
        world?.version,
        worldAssetUrl,
        worldId,
        worldUnityPackages
    ]);

    useEffect(() => {
        let active = true;

        if (!normalizedWorldId) {
            setWorld(null);
            setLoadStatus('error');
            setDetail(
                translateWorldDetail(
                    'dialog.world.empty.no_world_id_was_provided_for_this_dialog'
                )
            );
            return () => {
                active = false;
            };
        }

        setWorld(seedData ? worldProfileRepository.normalize(seedData) : null);
        setLoadStatus('running');
        setDetail('');

        worldProfileRepository
            .getWorldProfile({
                worldId: profileWorldId,
                dialog: true
            })
            .then((nextWorld) => {
                if (!active) {
                    return;
                }

                persistFavoriteWorldDetails(nextWorld);
                setWorld(nextWorld);
                setLoadStatus('ready');
            })
            .catch((error: unknown) => {
                if (!active) {
                    return;
                }

                if (seedData) {
                    setWorld(worldProfileRepository.normalize(seedData));
                    setLoadStatus('ready');
                    setDetail(
                        describeWorldLoadError(
                            error,
                            profileWorldId,
                            'dialog.world.error.failed_to_refresh_the_remote_world_snapshot'
                        )
                    );
                    return;
                }

                setWorld(null);
                setLoadStatus('error');
                setDetail(
                    describeWorldLoadError(
                        error,
                        profileWorldId,
                        'dialog.world.error.failed_to_load_the_world_profile'
                    )
                );
            });

        return () => {
            active = false;
        };
    }, [currentEndpoint, normalizedWorldId, profileWorldId, seedData]);

    useEffect(() => {
        let active = true;

        if (!profileWorldId) {
            setMemo('');
            return () => {
                active = false;
            };
        }

        setMemo('');
        const revision = memoRevisionRef.current;
        memoPersistenceRepository
            .getWorldMemo(profileWorldId)
            .then((entry) => {
                if (active && memoRevisionRef.current === revision) {
                    setMemo(entry?.memo || '');
                }
            })
            .catch(() => {
                if (active && memoRevisionRef.current === revision) {
                    setMemo('');
                }
            });

        return () => {
            active = false;
        };
    }, [memoRevisionRef, profileWorldId]);

    useEffect(() => {
        let active = true;

        if (!profileWorldId) {
            setHasPersistData(false);
            return () => {
                active = false;
            };
        }

        if (!currentUserId) {
            setHasPersistData(Boolean(world?.hasPersistData));
            return () => {
                active = false;
            };
        }

        worldProfileRepository
            .hasWorldPersistentData({
                userId: currentUserId,
                worldId: profileWorldId
            })
            .then((exists) => {
                if (active) {
                    setHasPersistData(exists);
                }
            })
            .catch(() => {
                if (active) {
                    setHasPersistData(Boolean(world?.hasPersistData));
                }
            });

        return () => {
            active = false;
        };
    }, [currentEndpoint, currentUserId, profileWorldId, world?.hasPersistData]);

    useEffect(() => {
        let active = true;

        if (!profileWorldId) {
            setPreviousInstances([]);
            return () => {
                active = false;
            };
        }

        gameLogRepository
            .getPreviousInstancesByWorldId({ worldId: profileWorldId })
            .then((rows) => {
                if (!active) {
                    return;
                }
                const values = Array.isArray(rows) ? rows : [];
                setPreviousInstances(values);
            })
            .catch(() => {
                if (active) {
                    setPreviousInstances([]);
                }
            });

        return () => {
            active = false;
        };
    }, [profileWorldId]);

    return {
        world,
        setWorld,
        loadStatus,
        detail,
        setDetail,
        memo,
        setMemo,
        previousInstances,
        setPreviousInstances,
        hasPersistData,
        setHasPersistData,
        worldSideData,
        setWorldSideData,
        newInstanceGroups
    };
}
