import { parseLocation } from '@/shared/utils/location';
import { useRuntimeStore } from '@/state/runtimeStore';

export function usePlayerListRuntime() {
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const currentUserEndpoint = useRuntimeStore(
        (state) => state.auth.currentUserEndpoint
    );
    const currentUserSnapshot = useRuntimeStore(
        (state) => state.auth.currentUserSnapshot
    );
    const gameLogLocation = useRuntimeStore(
        (state) => state.gameState.currentLocation || ''
    );
    const currentUserLocation = useRuntimeStore((state) => {
        return (
            state.gameState.currentLocation ||
            state.auth.currentUserSnapshot?.location ||
            ''
        );
    });
    const currentUserWorldId = useRuntimeStore(
        (state) =>
            parseLocation(state.gameState.currentLocation || '').worldId ||
            state.auth.currentUserSnapshot?.worldId ||
            ''
    );
    const currentLocationStartedAt = useRuntimeStore(
        (state) => state.gameState.currentLocationStartedAt
    );
    const isGameRunning = useRuntimeStore((state) =>
        Boolean(state.gameState.isGameRunning)
    );
    const addGameLogEventCount = useRuntimeStore(
        (state) => state.runtimeEvents.addGameLogEvent.count
    );
    const gameLogTailSyncedAt = useRuntimeStore(
        (state) => state.updateLoop.lastGameLogSyncAt
    );
    const runtimePlayerRows = useRuntimeStore(
        (state) => state.gameState.currentLocationPlayers
    );

    return {
        addGameLogEventCount,
        currentLocationStartedAt,
        currentUserEndpoint,
        currentUserId,
        currentUserLocation,
        currentUserSnapshot,
        currentUserWorldId,
        gameLogLocation,
        gameLogTailSyncedAt,
        isGameRunning,
        runtimePlayerRows
    };
}
