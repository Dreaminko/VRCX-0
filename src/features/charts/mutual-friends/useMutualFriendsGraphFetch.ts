import { useEffect, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import type { FriendRosterById } from '@/domain/friends/friendRosterTypes';
import {
    cancelMutualGraphFetch,
    refreshMutualGraphFetchStatus,
    startMutualGraphFetch,
    startMutualGraphFetchStatusPolling
} from '@/services/mutualGraphFetchService';
import { useRuntimeStore } from '@/state/runtimeStore';

import type { MutualFriendsFetchProgress } from './mutualFriendsTypes';

interface GraphFetchOptions {
    currentUserId: string;
    currentUserEndpoint?: string;
    friendsById: FriendRosterById;
    orderedFriendIds: string[];
    reloadSnapshot: (detail: string, ownerUserId: string) => Promise<void>;
    setDetail: (detail: string) => void;
}

export function useMutualFriendsGraphFetch({
    currentUserId,
    currentUserEndpoint = '',
    friendsById,
    orderedFriendIds,
    reloadSnapshot,
    setDetail
}: GraphFetchOptions) {
    const { t } = useTranslation();
    const lastHandledRunRef = useRef(0);
    const statusRunId = useRuntimeStore((state) => state.mutualGraph.runId);
    const statusName = useRuntimeStore((state) => state.mutualGraph.status);
    const statusOwnerUserId = useRuntimeStore(
        (state) => state.mutualGraph.ownerUserId
    );
    const processedFriends = useRuntimeStore(
        (state) => state.mutualGraph.processedFriends
    );
    const totalFriends = useRuntimeStore(
        (state) => state.mutualGraph.totalFriends
    );
    const cancelRequested = useRuntimeStore(
        (state) => state.mutualGraph.cancelRequested
    );
    const lastError = useRuntimeStore((state) => state.mutualGraph.lastError);

    useEffect(() => {
        refreshMutualGraphFetchStatus().catch(() => {});
    }, [currentUserId]);

    const isCurrentUserFetch =
        !statusOwnerUserId || statusOwnerUserId === currentUserId;
    const isFetching =
        isCurrentUserFetch &&
        (statusName === 'running' || statusName === 'cancelling');
    const fetchProgress = useMemo<MutualFriendsFetchProgress>(
        () => ({
            isFetching,
            processedFriends: isCurrentUserFetch ? processedFriends : 0,
            totalFriends: isCurrentUserFetch ? totalFriends : 0,
            cancelRequested: cancelRequested || statusName === 'cancelling'
        }),
        [
            cancelRequested,
            isCurrentUserFetch,
            isFetching,
            processedFriends,
            statusName,
            totalFriends
        ]
    );

    useEffect(() => {
        if (
            !isCurrentUserFetch ||
            !statusRunId ||
            statusRunId === lastHandledRunRef.current
        ) {
            return;
        }

        if (statusName === 'completed') {
            lastHandledRunRef.current = statusRunId;
            reloadSnapshot('', statusOwnerUserId).catch((error: unknown) => {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : t(
                              'view.charts.toast.failed_to_fetch_mutual_friends_graph'
                          )
                );
            });
            return;
        }

        if (statusName === 'cancelled') {
            lastHandledRunRef.current = statusRunId;
            return;
        }

        if (statusName === 'error') {
            lastHandledRunRef.current = statusRunId;
            setDetail(
                lastError ||
                    t('view.charts.toast.failed_to_fetch_mutual_friends_graph')
            );
        }
    }, [
        isCurrentUserFetch,
        lastError,
        reloadSnapshot,
        setDetail,
        statusName,
        statusOwnerUserId,
        statusRunId,
        t
    ]);

    async function handleFetchGraph() {
        if (!currentUserId || isFetching) {
            return;
        }
        const ownerUserId = currentUserId;

        const friendIds = orderedFriendIds
            .map((friendId) => friendsById[friendId]?.id)
            .filter((friendId): friendId is string => Boolean(friendId));
        if (!friendIds.length) {
            toast.info(
                t(
                    'view.charts.empty.no_friends_are_available_for_mutual_graph_fetching'
                )
            );
            return;
        }

        setDetail('');

        try {
            await startMutualGraphFetch({
                ownerUserId,
                endpoint: currentUserEndpoint,
                friendIds
            });
            startMutualGraphFetchStatusPolling();
            toast.info(t('view.charts.mutual_friend.prompt.message'));
        } catch (error) {
            const message =
                error instanceof Error
                    ? error.message
                    : t(
                          'view.charts.toast.failed_to_fetch_mutual_friends_graph'
                      );
            setDetail(message);
            toast.error(message);
        }
    }

    function handleCancelFetch() {
        if (!currentUserId) {
            return;
        }
        cancelMutualGraphFetch(currentUserId).catch((error: unknown) => {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'view.charts.toast.failed_to_fetch_mutual_friends_graph'
                      )
            );
        });
    }

    return {
        fetchProgress,
        handleCancelFetch,
        handleFetchGraph
    };
}
