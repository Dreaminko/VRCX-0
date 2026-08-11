import { useEffect, useMemo, useState } from 'react';

import type { EntityRecord } from '@/domain/entities/profileEntities';
import vrchatInstanceRepository from '@/repositories/vrchatInstanceRepository';
import { recordLocationHintsFromInstances } from '@/services/domainIngestionService';
import { parseLocation } from '@/shared/utils/location';

export interface WorldDialogInstanceDetailTarget {
    location: string;
    worldId: string;
    instanceId: string;
}

export interface WorldDialogInstanceDetailCacheEntry {
    endpoint: string;
    instance: EntityRecord;
}

type InstanceDetailResult = {
    location: string;
    instance: EntityRecord;
};

function isRecord(value: unknown): value is EntityRecord {
    return Boolean(value && typeof value === 'object');
}

function isInstanceDetailResult(
    value: { location: string; instance: EntityRecord | null } | null
): value is InstanceDetailResult {
    return Boolean(value?.instance);
}

export function useWorldDialogInstanceData({
    endpoint,
    targets
}: {
    endpoint: string;
    targets: WorldDialogInstanceDetailTarget[];
}) {
    const [detailsByLocation, setDetailsByLocation] = useState<
        Record<string, WorldDialogInstanceDetailCacheEntry>
    >({});
    const targetKey = useMemo(
        () =>
            targets
                .map((target) => target.location)
                .sort()
                .join('|'),
        [targets]
    );

    useEffect(() => {
        if (!targets.length) {
            setDetailsByLocation({});
            return;
        }

        let active = true;
        const targetLocations = new Set(
            targets.map((target) => target.location)
        );
        Promise.all(
            targets.map((target) =>
                vrchatInstanceRepository
                    .getInstance({
                        worldId: target.worldId,
                        instanceId: target.instanceId
                    })
                    .then((response) => ({
                        location: target.location,
                        instance: isRecord(response.json) ? response.json : null
                    }))
                    .catch((): null => null)
            )
        ).then((rawEntries) => {
            if (!active) {
                return;
            }
            recordLocationHintsFromInstances({
                endpoint,
                instances: rawEntries
                    .filter(isInstanceDetailResult)
                    .map((entry) => {
                        const parsedLocation = parseLocation(entry.location);
                        return {
                            ...entry.instance,
                            location: entry.location,
                            worldId: parsedLocation.worldId,
                            instanceId: parsedLocation.instanceId
                        };
                    })
            });
            setDetailsByLocation((current) => {
                const next: Record<
                    string,
                    WorldDialogInstanceDetailCacheEntry
                > = {};
                for (const location of targetLocations) {
                    const currentEntry = current[location];
                    if (currentEntry?.endpoint === endpoint) {
                        next[location] = currentEntry;
                    }
                }
                for (const entry of rawEntries) {
                    if (!entry?.instance) {
                        continue;
                    }
                    next[entry.location] = {
                        endpoint,
                        instance: entry.instance
                    };
                }
                return next;
            });
        });

        return () => {
            active = false;
        };
    }, [endpoint, targetKey, targets]);

    return { detailsByLocation };
}
