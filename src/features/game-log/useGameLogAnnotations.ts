import { useMemo } from 'react';

import { useFavoriteStore } from '@/state/favoriteStore';
import { useFriendRosterStore } from '@/state/friendRosterStore';

import {
    annotateGameLogSessionEvent,
    buildGameLogFavoriteIdSet,
    normalizeGameLogId
} from './gameLogRows';
import type { GameLogRow, GameLogSession } from './gameLogTypes';

export function useGameLogAnnotations({
    rows,
    sessions
}: {
    rows: GameLogRow[];
    sessions: GameLogSession[];
}) {
    const localFriendFavorites = useFavoriteStore(
        (state) => state.localFriendFavorites
    );
    const remoteFavoriteFriendIds = useFavoriteStore(
        (state) => state.favoriteFriendIds
    );
    const friendIdSignature = useFriendRosterStore((state) =>
        Object.keys(state.friendsById || {}).join(',')
    );
    const favoriteIdSet = useMemo(
        () =>
            buildGameLogFavoriteIdSet(
                remoteFavoriteFriendIds,
                localFriendFavorites
            ),
        [localFriendFavorites, remoteFavoriteFriendIds]
    );
    const friendIdSet = useMemo(
        () => new Set(friendIdSignature ? friendIdSignature.split(',') : []),
        [friendIdSignature]
    );
    const annotatedSessions = useMemo(
        () =>
            sessions.map((session) => ({
                ...session,
                events: (session.events ?? []).map((event) =>
                    annotateGameLogSessionEvent(
                        event,
                        favoriteIdSet,
                        friendIdSet
                    )
                )
            })),
        [favoriteIdSet, friendIdSet, sessions]
    );
    const annotatedRows = useMemo(
        () =>
            rows.map((row) => {
                const normalizedUserId = normalizeGameLogId(row?.userId);
                return {
                    ...row,
                    isFavorite: normalizedUserId
                        ? favoriteIdSet.has(normalizedUserId)
                        : false,
                    isFriend: normalizedUserId
                        ? friendIdSet.has(normalizedUserId)
                        : false
                };
            }),
        [favoriteIdSet, friendIdSet, rows]
    );

    return {
        annotatedRows,
        annotatedSessions
    };
}
