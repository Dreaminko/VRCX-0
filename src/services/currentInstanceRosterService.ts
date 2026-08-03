import {
    resolveRuntimeCurrentInstanceRoster,
    type CurrentInstanceRosterContext,
    type CurrentInstanceRosterSnapshot,
    type CurrentInstanceRuntimeRoster
} from '@/domain/instances/currentInstanceRoster';
import playerListPersistenceRepository, {
    type PlayerListContext as PersistenceRosterContext
} from '@/repositories/playerListPersistenceRepository';
import { normalizeString } from '@/shared/utils/string';

interface LoadCurrentInstanceRosterInput {
    currentLocation: unknown;
    currentLocationStartedAt?: unknown;
    currentUserId?: unknown;
    runtime?: CurrentInstanceRuntimeRoster;
}

function normalizeContext(
    context: PersistenceRosterContext
): CurrentInstanceRosterContext {
    return {
        ...context,
        playerCount: context.playerCount ?? 0
    };
}

export async function loadCurrentInstanceRoster({
    currentLocation,
    currentLocationStartedAt = '',
    currentUserId = '',
    runtime
}: LoadCurrentInstanceRosterInput): Promise<CurrentInstanceRosterSnapshot> {
    const normalizedLocation = normalizeString(currentLocation);
    if (runtime) {
        const runtimeSnapshot = resolveRuntimeCurrentInstanceRoster({
            requestedLocation: normalizedLocation,
            runtime
        });
        if (runtimeSnapshot) {
            return runtimeSnapshot;
        }
    }

    const snapshot =
        await playerListPersistenceRepository.getCurrentInstanceSnapshot({
            currentLocation: normalizedLocation,
            currentLocationStartedAt,
            currentUserId
        });
    return {
        context: normalizeContext(snapshot.context),
        players: snapshot.players
    };
}
