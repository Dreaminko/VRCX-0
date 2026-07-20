import { useEffect, useMemo, useState } from 'react';

import { isVrchatRequestError } from '@/repositories/vrchatRequest';
import worldProfileRepository from '@/repositories/worldProfileRepository';
import { persistWorldDetails } from '@/services/favoriteWorldCacheService';

import { hasDisplayableEntityDetail } from './favoriteEntityDetails';
import { normalizeFavoriteEntityId as normalizeEntityId } from './favoritesItems';

export type WorldAvailabilityStatus = 'public' | 'private' | 'deleted';
export type WorldAvailabilityById = Record<string, WorldAvailabilityStatus>;

const PROBE_CONCURRENCY = 3;

const EMPTY_AVAILABILITY: WorldAvailabilityById = {};

const statusByWorldId = new Map<string, WorldAvailabilityStatus>();
const inFlightWorldIds = new Set<string>();

export function clearWorldAvailabilityMemo() {
    statusByWorldId.clear();
    inFlightWorldIds.clear();
}

interface UseWorldAvailabilityProbeInput {
    favoriteWorldIds?: unknown;
    kind: unknown;
    remoteEntityDetailsData?: Record<string, unknown>;
    remoteEntityDetailsStatus?: unknown;
}

function normalizeIds(values: unknown): string[] {
    return Array.from(
        new Set(
            (Array.isArray(values) ? values : [])
                .map((value) => normalizeEntityId(value))
                .filter(Boolean)
        )
    );
}

async function runWithConcurrency<T>(
    items: T[],
    limit: number,
    worker: (item: T) => Promise<void>
): Promise<void> {
    let cursor = 0;
    async function drain(): Promise<void> {
        while (cursor < items.length) {
            const item = items[cursor];
            cursor += 1;
            await worker(item);
        }
    }
    await Promise.all(
        Array.from({ length: Math.min(limit, items.length) }, drain)
    );
}

export function useWorldAvailabilityProbe({
    favoriteWorldIds,
    kind,
    remoteEntityDetailsData,
    remoteEntityDetailsStatus
}: UseWorldAvailabilityProbeInput): WorldAvailabilityById {
    const normalizedIds = useMemo(
        () => normalizeIds(favoriteWorldIds),
        [favoriteWorldIds]
    );
    const missingIds = useMemo(() => {
        if (kind !== 'world' || remoteEntityDetailsStatus !== 'ready') {
            return [];
        }
        return normalizedIds.filter(
            (id) => !hasDisplayableEntityDetail(remoteEntityDetailsData?.[id])
        );
    }, [
        kind,
        normalizedIds,
        remoteEntityDetailsData,
        remoteEntityDetailsStatus
    ]);
    const missingKey = missingIds.join('|');

    const [availabilityById, setAvailabilityById] =
        useState<WorldAvailabilityById>(EMPTY_AVAILABILITY);

    useEffect(() => {
        if (!missingKey) {
            return;
        }
        let active = true;

        function recordStatus(
            worldId: string,
            status: WorldAvailabilityStatus
        ) {
            statusByWorldId.set(worldId, status);
            if (active) {
                setAvailabilityById((current) =>
                    current[worldId] === status
                        ? current
                        : { ...current, [worldId]: status }
                );
            }
        }

        async function probeWorld(worldId: string) {
            const memoized = statusByWorldId.get(worldId);
            if (memoized) {
                recordStatus(worldId, memoized);
                return;
            }
            if (inFlightWorldIds.has(worldId)) {
                return;
            }
            inFlightWorldIds.add(worldId);

            try {
                const world = await worldProfileRepository.fetchWorldProfile({
                    worldId
                });
                persistWorldDetails(world, worldId);
                recordStatus(
                    worldId,
                    world?.releaseStatus === 'public' ? 'public' : 'private'
                );
            } catch (error) {
                if (isVrchatRequestError(error) && error.status === 404) {
                    recordStatus(worldId, 'deleted');
                }
            } finally {
                inFlightWorldIds.delete(worldId);
            }
        }

        void runWithConcurrency(missingIds, PROBE_CONCURRENCY, probeWorld);

        return () => {
            active = false;
        };
    }, [missingKey]);

    return useMemo(() => {
        if (normalizedIds.length === 0) {
            return EMPTY_AVAILABILITY;
        }
        const next: WorldAvailabilityById = {};
        for (const id of normalizedIds) {
            const status = availabilityById[id] ?? statusByWorldId.get(id);
            if (status) {
                next[id] = status;
            }
        }
        return Object.keys(next).length > 0 ? next : EMPTY_AVAILABILITY;
    }, [availabilityById, normalizedIds]);
}
