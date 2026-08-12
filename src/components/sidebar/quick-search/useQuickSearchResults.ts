import { useMemo } from 'react';

import { useKnownUserFacts } from '@/lib/useKnownUser';
import { useFavoriteStore } from '@/state/favoriteStore';
import { useFriendRosterStore } from '@/state/friendRosterStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import type { QuickSearchCatalog } from '../quickSearchCatalog';
import { buildQuickSearchResults } from './quickSearchResultModel';
import { useWorldSearchDetails } from './useWorldSearchDetails';

const EMPTY_GROUP_INSTANCES: unknown[] = [];

export function useQuickSearchResults({
    catalog,
    normalizedQuery
}: {
    catalog: QuickSearchCatalog;
    normalizedQuery: string;
}) {
    const friendsById = useFriendRosterStore((state) => state.friendsById);
    const remoteFavoritesByObjectId = useFavoriteStore(
        (state) => state.remoteFavoritesByObjectId
    );
    const worldSearchDetailsById = useWorldSearchDetails(normalizedQuery);
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const currentEndpoint = useRuntimeStore(
        (state) => state.auth.currentUserEndpoint
    );
    const groupInstancesState = useRuntimeStore(
        (state) => state.groupInstances
    );
    const groupInstances =
        groupInstancesState.userId === currentUserId &&
        groupInstancesState.endpoint === currentEndpoint
            ? groupInstancesState.instances
            : EMPTY_GROUP_INSTANCES;
    const friendIds = useMemo(
        () => Object.keys(friendsById || {}).filter(Boolean),
        [friendsById]
    );
    const knownFriendUsersById = useKnownUserFacts(friendIds, {
        endpoint: currentEndpoint
    });

    return useMemo(
        () =>
            buildQuickSearchResults({
                catalog,
                normalizedQuery,
                currentUserId,
                friendsById,
                knownFriendUsersById,
                remoteFavoritesByObjectId,
                worldSearchDetailsById,
                groupInstances
            }),
        [
            catalog,
            currentUserId,
            friendsById,
            groupInstances,
            knownFriendUsersById,
            worldSearchDetailsById,
            normalizedQuery,
            remoteFavoritesByObjectId
        ]
    );
}
