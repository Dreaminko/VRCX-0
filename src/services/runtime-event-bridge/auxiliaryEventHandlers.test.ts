import { beforeEach, describe, expect, it } from 'vitest';

import { useFavoriteRevisionStore } from '@/state/favoriteRevisionStore';
import { useFavoriteStore } from '@/state/favoriteStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import {
    handleFavoritesChangedEvent,
    resetFavoritesChangedEventDelivery
} from './auxiliaryEventHandlers';
import type { FavoritesChangedEventPayload } from './types';

const scope = {
    ownerUserId: 'usr_self',
    endpoint: 'https://api.vrchat.cloud/api/1'
};

function favoritesChanged(
    overrides: Partial<FavoritesChangedEventPayload> = {}
): FavoritesChangedEventPayload {
    return {
        ...scope,
        kind: 'friend',
        local: true,
        remote: false,
        changes: [],
        requiresRefresh: false,
        ...overrides
    };
}

describe('handleFavoritesChangedEvent', () => {
    beforeEach(() => {
        resetFavoritesChangedEventDelivery();
        useFavoriteStore.getState().resetFavorites();
        useFavoriteStore.getState().setFavoritesSnapshot({
            currentUserId: scope.ownerUserId
        });
        useFavoriteRevisionStore.setState({
            revision: 0,
            localWorldRevision: 0,
            lastAttemptedRevision: 0,
            pendingRemote: false,
            pendingUnknown: false
        });
        useRuntimeStore.getState().setAuthBootstrap({
            currentUserId: scope.ownerUserId,
            currentUserEndpoint: scope.endpoint
        });
    });

    it('applies an exact local delta without scheduling a persistence reload', () => {
        handleFavoritesChangedEvent(
            favoritesChanged({
                changes: [
                    {
                        type: 'localAdded',
                        kind: 'friend',
                        entityId: 'usr_friend',
                        groupName: 'Close'
                    }
                ]
            })
        );

        expect(useFavoriteStore.getState().localFriendFavorites).toEqual({
            Close: ['usr_friend']
        });
        expect(useFavoriteRevisionStore.getState()).toMatchObject({
            revision: 1,
            pendingRemote: false,
            pendingUnknown: false
        });
    });

    it('ignores a delta emitted for a replaced account scope', () => {
        handleFavoritesChangedEvent(
            favoritesChanged({
                ownerUserId: 'usr_previous',
                changes: [
                    {
                        type: 'localAdded',
                        kind: 'friend',
                        entityId: 'usr_friend',
                        groupName: 'Close'
                    }
                ]
            })
        );

        expect(useFavoriteStore.getState().localFriendFavorites).toEqual({});
        expect(useFavoriteRevisionStore.getState().revision).toBe(0);
    });

    it('replays a current event after the matching favorites baseline is installed', () => {
        useFavoriteStore.getState().setFavoritesLoading('usr_previous');

        handleFavoritesChangedEvent(
            favoritesChanged({
                changes: [
                    {
                        type: 'localAdded',
                        kind: 'friend',
                        entityId: 'usr_friend',
                        groupName: 'Close'
                    }
                ]
            })
        );

        expect(useFavoriteStore.getState().localFriendFavorites).toEqual({});
        expect(useFavoriteRevisionStore.getState().revision).toBe(0);

        useFavoriteStore.getState().setFavoritesSnapshot({
            currentUserId: scope.ownerUserId,
            localFriendFavorites: {}
        });

        expect(useFavoriteStore.getState().localFriendFavorites).toEqual({
            Close: ['usr_friend']
        });
        expect(useFavoriteRevisionStore.getState().revision).toBe(1);
    });

    it('stops replaying queued events when the first delta synchronously changes auth', () => {
        useFavoriteStore.getState().setFavoritesLoading('usr_previous');
        handleFavoritesChangedEvent(
            favoritesChanged({
                changes: [
                    {
                        type: 'localAdded',
                        kind: 'friend',
                        entityId: 'usr_first',
                        groupName: 'Close'
                    }
                ]
            })
        );
        handleFavoritesChangedEvent(
            favoritesChanged({
                changes: [
                    {
                        type: 'localAdded',
                        kind: 'friend',
                        entityId: 'usr_second',
                        groupName: 'Close'
                    }
                ]
            })
        );
        const unsubscribe = useFavoriteStore.subscribe((state) => {
            if (state.localFriendFavorites.Close?.includes('usr_first')) {
                useRuntimeStore.getState().setAuthBootstrap({
                    currentUserId: 'usr_other',
                    currentUserEndpoint: scope.endpoint
                });
            }
        });

        useFavoriteStore.getState().setFavoritesSnapshot({
            currentUserId: scope.ownerUserId,
            localFriendFavorites: {}
        });
        unsubscribe();

        expect(useFavoriteStore.getState().localFriendFavorites).toEqual({
            Close: ['usr_first']
        });
        expect(useFavoriteRevisionStore.getState().revision).toBe(1);
    });

    it('collapses an overflowing queue into one refresh invalidation', () => {
        useFavoriteStore.getState().setFavoritesLoading('usr_previous');
        for (let index = 0; index < 65; index += 1) {
            handleFavoritesChangedEvent(
                favoritesChanged({
                    local: false,
                    remote: true,
                    changes: [
                        {
                            type: 'remoteRemoved',
                            objectId: `usr_${index}`
                        }
                    ]
                })
            );
        }

        useFavoriteStore.getState().setFavoritesSnapshot({
            currentUserId: scope.ownerUserId
        });

        expect(useFavoriteRevisionStore.getState()).toMatchObject({
            revision: 1,
            pendingRemote: true
        });
    });
});
