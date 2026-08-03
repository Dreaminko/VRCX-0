import { useMemo } from 'react';

import { buildPlayerSourceRows } from './playerListRows';
import type {
    PlayerListContext,
    PlayerListCurrentUserSnapshot,
    PlayerListRosterRow,
    PlayerListSourceRow
} from './playerListTypes';

export function usePlayerListSourceRows({
    context,
    currentLocationStartedAt,
    currentUserId,
    currentUserLocation,
    currentUserSnapshot,
    isGameRunning,
    playerRows,
    runtimeRosterAvailable,
    runtimePlayerRows
}: {
    context: PlayerListContext;
    currentLocationStartedAt?: string | null;
    currentUserId?: string | null;
    currentUserLocation?: string | null;
    currentUserSnapshot?: PlayerListCurrentUserSnapshot | null;
    isGameRunning: boolean;
    playerRows?: readonly PlayerListRosterRow[];
    runtimeRosterAvailable?: boolean;
    runtimePlayerRows?: readonly PlayerListRosterRow[];
}): PlayerListSourceRow[] {
    return useMemo(() => {
        return buildPlayerSourceRows({
            context,
            currentLocationStartedAt,
            currentUserId,
            currentUserLocation,
            currentUserSnapshot,
            isGameRunning,
            playerRows,
            runtimePlayerRows,
            runtimeRosterAvailable
        });
    }, [
        context,
        currentLocationStartedAt,
        currentUserId,
        currentUserLocation,
        currentUserSnapshot,
        isGameRunning,
        playerRows,
        runtimeRosterAvailable,
        runtimePlayerRows
    ]);
}
