import type { EntityRecord } from '@/domain/entities/profileEntities';
import {
    entityQueryPolicies,
    fetchCachedData,
    queryKeys
} from '@/lib/entityQueryCache';
import { commands } from '@/platform/tauri/bindings';

export type UserDialogDataTab =
    | 'mutual'
    | 'groups'
    | 'worlds'
    | 'favorite-worlds'
    | 'avatars';

export type UserDialogRepositories = {
    avatarSearchProviderRepository: {
        getConfig(): Promise<{ enabled: boolean; selectedProvider: string }>;
        search(input: {
            provider: string;
            query: string;
        }): Promise<{ avatars: unknown[] }>;
    };
    myAvatarRepository: {
        getMyAvatars(input: Record<string, unknown>): Promise<unknown[]>;
    };
    groupProfileRepository: {
        getUserGroups(input: Record<string, unknown>): Promise<unknown[]>;
    };
    userProfileRepository: {
        getAllMutualFriends(input: Record<string, unknown>): Promise<unknown[]>;
    };
    vrchatFavoriteRepository: {
        getAllFavoriteGroups(
            input: Record<string, unknown>
        ): Promise<unknown[]>;
        getAllFavoriteWorlds(
            input: Record<string, unknown>
        ): Promise<unknown[]>;
    };
    worldProfileRepository: {
        getAllWorldsByUser(input: Record<string, unknown>): Promise<unknown[]>;
    };
};

type UserDialogTabCounts = {
    groups?: number;
    worlds?: number;
    'favorite-worlds'?: number;
    avatars?: number;
};

function isRecord(value: unknown): value is EntityRecord {
    return Boolean(value && typeof value === 'object');
}

function recordRows(value: unknown): EntityRecord[] {
    return Array.isArray(value) ? value.filter(isRecord) : [];
}

const userDialogDataTabs: ReadonlySet<unknown> = new Set<UserDialogDataTab>([
    'mutual',
    'groups',
    'worlds',
    'favorite-worlds',
    'avatars'
]);

export function isUserDialogDataTab(tab: unknown): tab is UserDialogDataTab {
    return userDialogDataTabs.has(tab);
}

export function userDialogDataKeyForTab(tab: UserDialogDataTab) {
    return tab === 'favorite-worlds' ? 'favoriteWorlds' : tab;
}

export async function loadUserDialogTabCounts({
    userId,
    endpoint,
    currentUserId,
    effectiveAvatarReleaseStatus,
    force = false
}: {
    userId: string;
    endpoint: string;
    currentUserId: string;
    effectiveAvatarReleaseStatus: string;
    force?: boolean;
}): Promise<UserDialogTabCounts> {
    if (!userId) {
        return {};
    }
    return fetchCachedData({
        queryKey: queryKeys.userDialogTabCounts(
            {
                userId,
                currentUserId,
                avatarReleaseStatus: effectiveAvatarReleaseStatus
            },
            endpoint
        ),
        policy: entityQueryPolicies.userDialogTabCounts,
        force,
        queryFn: async () => {
            const counts = await commands.appUserDialogTabCountsGet({
                userId,
                avatarReleaseStatus: effectiveAvatarReleaseStatus,
                force
            });

            return {
                groups: counts.groups ?? undefined,
                worlds: counts.worlds ?? undefined,
                'favorite-worlds': counts.favoriteWorlds ?? undefined,
                avatars: counts.avatars ?? undefined
            };
        }
    });
}

export async function loadUserDialogTabData({
    tab,
    userId,
    endpoint,
    currentUserId,
    currentAvatarId = '',
    previousAvatarSwapTime = 0,
    worldSort,
    worldOrder,
    repositories
}: {
    tab: string;
    userId: string;
    endpoint?: string;
    currentUserId?: string;
    currentAvatarId?: string;
    previousAvatarSwapTime?: number;
    avatarSort?: string;
    effectiveAvatarReleaseStatus?: string;
    worldSort?: string;
    worldOrder?: string;
    repositories: UserDialogRepositories;
}): Promise<{ rows: EntityRecord[]; favoriteWorldGroups: EntityRecord[] }> {
    if (!isUserDialogDataTab(tab)) {
        return { rows: [], favoriteWorldGroups: [] };
    }

    if (tab === 'mutual') {
        const rows =
            await repositories.userProfileRepository.getAllMutualFriends({
                userId,
                endpoint
            });
        return { rows: recordRows(rows), favoriteWorldGroups: [] };
    }

    if (tab === 'groups') {
        const rows = await repositories.groupProfileRepository.getUserGroups({
            userId,
            endpoint
        });
        return { rows: recordRows(rows), favoriteWorldGroups: [] };
    }

    if (tab === 'worlds') {
        const rows =
            await repositories.worldProfileRepository.getAllWorldsByUser({
                userId,
                endpoint,
                sort: worldSort,
                order: worldOrder,
                releaseStatus: userId === currentUserId ? 'all' : 'public'
            });
        return { rows: recordRows(rows), favoriteWorldGroups: [] };
    }

    if (tab === 'avatars') {
        if (userId === currentUserId) {
            const rows = await repositories.myAvatarRepository.getMyAvatars({
                endpoint,
                currentUserId,
                currentAvatarId,
                previousAvatarSwapTime
            });
            return { rows: recordRows(rows), favoriteWorldGroups: [] };
        }

        const providerConfig =
            await repositories.avatarSearchProviderRepository.getConfig();
        if (!providerConfig.enabled || !providerConfig.selectedProvider) {
            return { rows: [], favoriteWorldGroups: [] };
        }

        const response =
            await repositories.avatarSearchProviderRepository.search({
                provider: providerConfig.selectedProvider,
                query: userId
            });
        return {
            rows: response.avatars.filter(
                (avatar): avatar is EntityRecord =>
                    isRecord(avatar) && avatar.authorId === userId
            ),
            favoriteWorldGroups: []
        };
    }

    const favoriteGroups =
        await repositories.vrchatFavoriteRepository.getAllFavoriteGroups({
            endpoint,
            ownerId: userId
        });
    const worldGroups = favoriteGroups
        .filter(isRecord)
        .filter((group) => group.type === 'world');
    const worldListResults = await Promise.allSettled(
        worldGroups.map(async (group) => {
            const worlds =
                await repositories.vrchatFavoriteRepository.getAllFavoriteWorlds(
                    {
                        endpoint,
                        ownerId: userId,
                        userId,
                        tag: group.name
                    }
                );
            return recordRows(worlds).map((world) => ({
                ...world,
                $favoriteGroup: group.displayName || group.name,
                $favoriteGroupKey: group.name
            }));
        })
    );
    return {
        rows: worldListResults.flatMap((result) =>
            result.status === 'fulfilled' ? result.value : []
        ),
        favoriteWorldGroups: recordRows(worldGroups)
    };
}
