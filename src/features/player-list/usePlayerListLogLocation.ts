import { useEffect, useState } from 'react';

import { getCurrentLogLocation } from '@/services/gameLogWatcherService';
import { normalizeString } from '@/shared/utils/string';

import { isLiveLocation } from './playerListRows';

type LogLocationSnapshot = {
    createdAt: string;
    fileName: string;
    location: string;
    worldName: string;
};

function normalizeLogLocationSnapshot(
    snapshot: unknown
): LogLocationSnapshot | null {
    if (!snapshot || typeof snapshot !== 'object') {
        return null;
    }
    const source = snapshot as Record<string, unknown>;

    const location = normalizeString(source.location);
    if (!isLiveLocation(location)) {
        return null;
    }

    return {
        createdAt:
            normalizeString(source.createdAt) || new Date().toISOString(),
        fileName: normalizeString(source.fileName),
        location,
        worldName: normalizeString(source.worldName)
    };
}

export function usePlayerListLogLocation({
    addGameLogEventCount,
    currentUserId,
    currentUserLocation,
    isGameRunning
}: {
    addGameLogEventCount?: unknown;
    currentUserId?: unknown;
    currentUserLocation?: unknown;
    isGameRunning: boolean;
}) {
    const [logLocationSnapshot, setLogLocationSnapshot] =
        useState<ReturnType<typeof normalizeLogLocationSnapshot>>(null);

    useEffect(() => {
        let active = true;

        if (currentUserLocation || !isGameRunning) {
            setLogLocationSnapshot(null);
            return () => {
                active = false;
            };
        }

        if (logLocationSnapshot) {
            return () => {
                active = false;
            };
        }

        getCurrentLogLocation()
            .then((snapshot) => {
                if (!active) {
                    return;
                }

                const normalized = normalizeLogLocationSnapshot(snapshot);
                const normalizedKey = JSON.stringify(normalized || null);
                setLogLocationSnapshot((previous) =>
                    JSON.stringify(previous || null) === normalizedKey
                        ? previous
                        : normalized
                );
            })
            .catch(() => {
                if (!active) {
                    return;
                }

                setLogLocationSnapshot(null);
            });

        return () => {
            active = false;
        };
    }, [
        addGameLogEventCount,
        currentUserId,
        currentUserLocation,
        isGameRunning,
        logLocationSnapshot
    ]);

    return logLocationSnapshot;
}
