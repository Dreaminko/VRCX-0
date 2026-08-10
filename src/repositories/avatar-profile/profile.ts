import type { AvatarProfileRecord } from '@/domain/entities/profileEntities';
import {
    entityQueryPolicies,
    fetchCachedData,
    queryKeys
} from '@/lib/entityQueryCache';
import {
    commands,
    type VrchatAvatarListByUserInput
} from '@/platform/tauri/bindings';
import { DEFAULT_VRCHAT_API_ENDPOINT } from '@/shared/vrchatEndpoint';

import avatarLocalRepository from '../avatarLocalRepository';
import memoPersistenceRepository from '../memoPersistenceRepository';
import { VRCHAT_API_DEFAULT_PAGE_SIZE } from '../paginationConstants';
import { normalize, normalizeLocalTags } from './normalization';
import {
    collectPages,
    isRecord,
    normalizeEntityId,
    normalizeString,
    parseInteger,
    unwrapVrchatAvatarResponse
} from './shared';
import type {
    AvatarListOptions,
    AvatarProfileExtras,
    AvatarProfileInput,
    AvatarRecord,
    AvatarStyleRecord,
    AvatarStylesInput
} from './types';

async function getLocalMetadata(
    avatarId: string,
    currentUserId: unknown
): Promise<AvatarProfileExtras> {
    const [localTags, timeSpentEntry, memoEntry] = await Promise.all([
        avatarLocalRepository
            .getAvatarTags(avatarId)
            .catch(
                (): Awaited<
                    ReturnType<typeof avatarLocalRepository.getAvatarTags>
                > => []
            ),
        currentUserId
            ? avatarLocalRepository
                  .getAvatarTimeSpent(currentUserId, avatarId)
                  .catch(
                      (): Awaited<
                          ReturnType<
                              typeof avatarLocalRepository.getAvatarTimeSpent
                          >
                      > | null => null
                  )
            : Promise.resolve(null),
        memoPersistenceRepository
            .getAvatarMemo(avatarId)
            .catch(
                (): Awaited<
                    ReturnType<typeof memoPersistenceRepository.getAvatarMemo>
                > | null => null
            )
    ]);

    return {
        cachedAvatar: null,
        localTags: normalizeLocalTags(localTags),
        timeSpent: parseInteger(timeSpentEntry?.timeSpent),
        memo: normalizeString(memoEntry?.memo)
    };
}

async function requestAvatar(
    avatarId: string,
    full: boolean,
    fresh: boolean
): Promise<AvatarRecord> {
    const avatar = await commands.appAvatarGet({ avatarId, full, fresh });
    if (!isRecord(avatar)) {
        throw new Error(`Avatar request failed: ${avatarId}`);
    }
    return avatar;
}

export async function getAvatarProfile({
    avatarId,
    force = false,
    dialog = false,
    allowLocalFallback = true,
    currentUserId = ''
}: AvatarProfileInput): Promise<AvatarProfileRecord> {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.getAvatarProfile requires an avatar id.'
        );
    }

    const localMetadataPromise = dialog
        ? getLocalMetadata(normalizedAvatarId, currentUserId)
        : Promise.resolve<AvatarProfileExtras>({
              cachedAvatar: null,
              localTags: [],
              timeSpent: 0,
              memo: ''
          });

    try {
        const [json, localMetadata] = await Promise.all([
            requestAvatar(normalizedAvatarId, dialog || force, force),
            localMetadataPromise
        ]);

        return normalize(json, {
            ...localMetadata,
            cachedAvatar: json
        });
    } catch (error) {
        if (allowLocalFallback) {
            const [cachedAvatar, localMetadata] = await Promise.all([
                requestAvatar(normalizedAvatarId, false, false).catch(
                    () => null
                ),
                localMetadataPromise
            ]);
            if (cachedAvatar) {
                return normalize(cachedAvatar, {
                    ...localMetadata,
                    cachedAvatar
                });
            }
        }

        throw error;
    }
}

export async function findAvatarByImageUrl(
    imageUrl: unknown
): Promise<AvatarProfileRecord | null> {
    const normalizedImageUrl = normalizeString(imageUrl);
    if (!normalizedImageUrl) {
        return null;
    }
    const avatar = await commands.appAvatarFindByImageUrl(normalizedImageUrl);
    if (!isRecord(avatar)) {
        return null;
    }
    return normalize(avatar);
}

export async function getAvatarsByUser({
    userId,
    user = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0,
    sort = 'updated',
    order = 'descending',
    releaseStatus = 'all'
}: AvatarListOptions = {}): Promise<AvatarProfileRecord[]> {
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedUserId) {
        throw new Error(
            'AvatarProfileRepository.getAvatarsByUser requires a user id.'
        );
    }

    const input = {
        userId: normalizedUserId,
        user,
        n,
        offset,
        sort,
        order,
        releaseStatus
    } satisfies VrchatAvatarListByUserInput;
    const response = unwrapVrchatAvatarResponse<AvatarRecord[]>(
        await commands.appVrchatAvatarListByUserGet(input),
        'avatars'
    );
    return Array.isArray(response.json)
        ? response.json.map((avatar) => normalize(avatar))
        : [];
}

export async function getAllAvatarsByUser({
    userId,
    user = '',
    sort = 'updated',
    order = 'descending',
    releaseStatus = 'all'
}: Omit<AvatarListOptions, 'n' | 'offset'> = {}): Promise<
    AvatarProfileRecord[]
> {
    return collectPages(({ n, offset }) =>
        getAvatarsByUser({
            userId,
            user,
            n,
            offset,
            sort,
            order,
            releaseStatus
        })
    );
}

export async function getAvatarStyles({
    force = false
}: AvatarStylesInput = {}): Promise<AvatarStyleRecord[]> {
    return fetchCachedData({
        queryKey: queryKeys.avatarStyles(DEFAULT_VRCHAT_API_ENDPOINT),
        policy: entityQueryPolicies.avatarStyles,
        force,
        queryFn: async () => {
            const response = unwrapVrchatAvatarResponse<AvatarStyleRecord[]>(
                await commands.appVrchatAvatarStylesGet(),
                'avatarStyles'
            );
            return Array.isArray(response.json) ? response.json : [];
        }
    });
}
