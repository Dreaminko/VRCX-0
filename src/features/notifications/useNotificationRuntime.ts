import { useMemo } from 'react';

import { checkCanInvite } from '@/shared/utils/invite';
import { useRuntimeStore } from '@/state/runtimeStore';

import {
    buildCachedInstanceMap,
    resolveCurrentInviteLocation
} from './notificationRows';

const EMPTY_GROUP_INSTANCES: unknown[] = [];

export function useNotificationRuntime() {
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const endpoint = useRuntimeStore((state) => state.auth.currentUserEndpoint);
    const currentUserLocationTag = useRuntimeStore(
        (state) => state.auth.currentUserSnapshot?.$locationTag
    );
    const currentUserLocation = useRuntimeStore(
        (state) => state.auth.currentUserSnapshot?.location
    );
    const isLocalUserVrcPlusSupporter = useRuntimeStore((state) => {
        const tags = state.auth.currentUserSnapshot?.tags;
        return Boolean(
            state.auth.currentUserSnapshot?.$isVRCPlus ||
            (Array.isArray(tags) && tags.includes('system_supporter')) ||
            globalThis.$debug?.debugVrcPlus
        );
    });
    const currentLocation = useRuntimeStore(
        (state) => state.gameState.currentLocation
    );
    const currentDestination = useRuntimeStore(
        (state) => state.gameState.currentDestination
    );
    const isGameRunning = useRuntimeStore(
        (state) => state.gameState.isGameRunning
    );
    const groupInstancesEndpoint = useRuntimeStore(
        (state) => state.groupInstances.endpoint
    );
    const groupInstancesUserId = useRuntimeStore(
        (state) => state.groupInstances.userId
    );
    const groupInstances = useRuntimeStore(
        (state) => state.groupInstances.instances
    );

    const groupInstanceRows =
        groupInstancesUserId === currentUserId &&
        groupInstancesEndpoint === endpoint
            ? groupInstances
            : EMPTY_GROUP_INSTANCES;
    const gameState = useMemo(
        () => ({
            currentDestination,
            currentLocation,
            isGameRunning
        }),
        [currentDestination, currentLocation, isGameRunning]
    );
    const currentUserSnapshot = useMemo(
        () => ({
            $locationTag: currentUserLocationTag,
            location: currentUserLocation
        }),
        [currentUserLocation, currentUserLocationTag]
    );
    const currentInviteLocation = useMemo(
        () => resolveCurrentInviteLocation(gameState, currentUserSnapshot),
        [gameState, currentUserSnapshot]
    );
    const cachedInstances = useMemo(
        () => buildCachedInstanceMap(groupInstanceRows),
        [groupInstanceRows]
    );
    const canInviteFromCurrentLocation = useMemo(
        () =>
            checkCanInvite(currentInviteLocation, {
                cachedInstances,
                currentUserId: currentUserId ?? '',
                lastLocationStr: currentInviteLocation
            }),
        [cachedInstances, currentInviteLocation, currentUserId]
    );

    return {
        canInviteFromCurrentLocation,
        currentInviteLocation,
        currentUserId,
        endpoint,
        isLocalUserVrcPlusSupporter
    };
}
