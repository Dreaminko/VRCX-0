import { describe, expect, it } from 'vitest';

import {
    buildFavoriteAvatarDetailIds,
    buildFavoriteAvatarTags,
    buildFavoriteFriendFactIds,
    buildFavoriteRemoteGroupEntityIds,
    selectFavoritesCollectionsState
} from './favoritesCollectionsState';

describe('favorites collections state helpers', () => {
    it('builds on-demand avatar ids from remote and local favorites', () => {
        expect(
            buildFavoriteAvatarDetailIds({
                kind: 'avatar',
                favoriteAvatarIds: ['avtr_remote', ' avtr_shared '],
                localAvatarFavorites: {
                    Local: ['avtr_local', 'avtr_shared', '']
                }
            })
        ).toEqual(['avtr_remote', 'avtr_shared', 'avtr_local']);
        expect(
            buildFavoriteAvatarDetailIds({
                kind: 'world',
                favoriteAvatarIds: ['avtr_ignored']
            })
        ).toEqual([]);
    });

    it('builds unique friend fact ids from remote and local groups', () => {
        expect(
            buildFavoriteFriendFactIds({
                kind: 'friend',
                groupedFavoriteFriendIdsByGroupKey: {
                    'friend:group_0': ['usr_a', 'usr_b', ''],
                    'friend:group_1': ['usr_a', 42]
                },
                localFriendFavorites: {
                    Local: ['usr_c', 'usr_b', null]
                }
            })
        ).toEqual(['usr_a', 'usr_b', '42', 'usr_c']);

        expect(
            buildFavoriteFriendFactIds({
                kind: 'world',
                groupedFavoriteFriendIdsByGroupKey: {
                    'friend:group_0': ['usr_a']
                }
            })
        ).toEqual([]);
    });

    it('builds unique avatar tags only for avatar collections', () => {
        expect(
            buildFavoriteAvatarTags({
                kind: 'avatar',
                remoteFavoritesById: {
                    one: { type: 'avatar', tags: ['author_tag_foo'] },
                    two: { type: 'avatar', tags: ['author_tag_foo'] },
                    three: { type: 'world', tags: ['author_tag_world'] },
                    four: { type: 'avatar', tags: ['  '] }
                }
            })
        ).toEqual(['author_tag_foo']);

        expect(
            buildFavoriteAvatarTags({
                kind: 'friend',
                remoteFavoritesById: {
                    one: { type: 'avatar', tags: ['author_tag_foo'] }
                }
            })
        ).toEqual([]);
    });

    it('selects world and VRC+ ids for one remote group', () => {
        const remoteFavoritesById = {
            one: {
                type: 'world',
                favoriteId: 'wrld_1',
                $groupKey: 'world:worlds1'
            },
            two: {
                type: 'world',
                favoriteId: ' wrld_2 ',
                $groupKey: 'world:worlds2'
            },
            three: {
                type: 'vrcPlusWorld',
                favoriteId: 'wrld_plus',
                $groupKey: 'vrcPlusWorld:vrcPlusWorlds1'
            },
            avatar: {
                type: 'avatar',
                favoriteId: 'avtr_ignored',
                $groupKey: 'world:worlds1'
            }
        };

        expect(
            buildFavoriteRemoteGroupEntityIds({
                groupKey: ' world:worlds1 ',
                kind: 'world',
                remoteFavoritesById
            })
        ).toEqual(['wrld_1']);
        expect(
            buildFavoriteRemoteGroupEntityIds({
                groupKey: 'vrcPlusWorld:vrcPlusWorlds1',
                kind: 'world',
                remoteFavoritesById
            })
        ).toEqual(['wrld_plus']);
    });

    it('selects only the favorite state needed for the active kind', () => {
        const state = {
            loadStatus: 'ready',
            detail: '',
            lastLoadedAt: '2026-08-11T00:00:00.000Z',
            favoritesSortOrder: ['fav_1'],
            remoteFavoritesById: { fav_1: { favoriteId: 'wrld_1' } },
            favoriteFriendGroups: [{ key: 'friend:group_0' }],
            favoriteWorldGroups: [{ key: 'world:group_0' }],
            favoriteAvatarGroups: [{ key: 'avatar:group_0' }],
            groupedFavoriteFriendIdsByGroupKey: {
                'friend:group_0': ['usr_a']
            },
            localAvatarFavorites: { Avatars: ['avtr_1'] },
            localFriendFavorites: { Friends: ['usr_a'] },
            localAvatarFavoriteGroups: ['Avatars'],
            localFriendFavoriteGroups: ['Friends'],
            favoriteWorldIds: ['wrld_1'],
            favoriteAvatarIds: ['avtr_1']
        };

        expect(selectFavoritesCollectionsState('friend')(state)).toMatchObject({
            favoriteFriendGroups: [{ key: 'friend:group_0' }],
            favoriteWorldGroups: [],
            favoriteAvatarGroups: [],
            remoteFavoritesById: {},
            localFriendFavorites: { Friends: ['usr_a'] },
            localAvatarFavorites: {}
        });
        expect(selectFavoritesCollectionsState('world')(state)).toMatchObject({
            favoriteFriendGroups: [],
            favoriteWorldGroups: [{ key: 'world:group_0' }],
            remoteFavoritesById: { fav_1: { favoriteId: 'wrld_1' } }
        });
    });
});
