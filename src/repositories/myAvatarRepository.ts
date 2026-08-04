import {
    entityQueryPolicies,
    fetchCachedData,
    queryKeys
} from '@/lib/entityQueryCache';
import { commands } from '@/platform/tauri/bindings';
import { DEFAULT_VRCHAT_API_ENDPOINT } from '@/shared/vrchatEndpoint';

import avatarCacheRepository from './avatarCacheRepository';
import type { AvatarStyleRecord } from './avatarProfileRepository';
import { unwrapVrchatResponse } from './vrchatRequest';

type AvatarRecord = Record<string, unknown>;
type VrchatApiResult = {
    status: number;
    data: unknown;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function unwrapVrchatAvatarResponse<TJson = unknown>(
    response: VrchatApiResult,
    path: string
) {
    return unwrapVrchatResponse<TJson>(response, path, {
        fallbackMessage: 'VRChat avatar request failed'
    });
}

interface AvatarByIdOptions {
    avatarId?: unknown;
}

interface MyAvatarsOptions {
    currentUserId?: string;
    currentAvatarId?: string;
    previousAvatarSwapTime?: number;
}

interface AvatarTagEntry {
    tag: string;
    color?: string | null;
}

export type MyAvatarRecord = AvatarRecord & {
    id: string;
    name?: string;
    $tags: AvatarTagEntry[];
    $timeSpent: number;
};

interface UpdateAvatarTagsInput {
    avatarId?: unknown;
    previousTags?: AvatarTagEntry[];
    nextTags?: AvatarTagEntry[];
}

interface SaveAvatarInput {
    avatarId?: unknown;
    params?: Record<string, unknown>;
}

interface AvatarIdInput {
    avatarId?: unknown;
}

interface AvatarStylesInput {
    force?: boolean;
}

function avatarIdFromValue(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

async function getMyAvatarById({ avatarId }: AvatarByIdOptions = {}) {
    const normalizedAvatarId = avatarIdFromValue(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'MyAvatarRepository.getMyAvatarById requires an avatar id.'
        );
    }

    const avatar = await commands.appMyAvatarByIdGet({
        avatarId: normalizedAvatarId
    });
    return isRecord(avatar) ? avatar : null;
}

async function getMyAvatars({
    currentAvatarId = '',
    previousAvatarSwapTime = 0
}: MyAvatarsOptions = {}) {
    const avatars = await commands.appMyAvatarsGet({
        currentAvatarId,
        previousAvatarSwapTime: Number.isFinite(previousAvatarSwapTime)
            ? previousAvatarSwapTime
            : 0
    });
    return (Array.isArray(avatars) ? avatars : []).filter(
        (avatar): avatar is MyAvatarRecord => isRecord(avatar)
    );
}

async function updateAvatarTags({
    avatarId,
    previousTags = [],
    nextTags = []
}: UpdateAvatarTagsInput) {
    const normalizedAvatarId =
        typeof avatarId === 'string' ? avatarId.trim() : '';
    if (!normalizedAvatarId) {
        throw new Error(
            'MyAvatarRepository.updateAvatarTags requires an avatar id.'
        );
    }

    const previousMap = new Map(
        (Array.isArray(previousTags) ? previousTags : [])
            .filter(
                (entry): entry is AvatarTagEntry =>
                    typeof entry?.tag === 'string' && Boolean(entry.tag.trim())
            )
            .map((entry) => [
                entry.tag.trim(),
                { tag: entry.tag.trim(), color: entry.color || null }
            ])
    );
    const nextMap = new Map(
        (Array.isArray(nextTags) ? nextTags : [])
            .filter(
                (entry): entry is AvatarTagEntry =>
                    typeof entry?.tag === 'string' && Boolean(entry.tag.trim())
            )
            .map((entry) => [
                entry.tag.trim(),
                { tag: entry.tag.trim(), color: entry.color || null }
            ])
    );

    const nextEntries = Array.from(nextMap.values());
    const previousEntries = Array.from(previousMap.values());
    if (JSON.stringify(previousEntries) !== JSON.stringify(nextEntries)) {
        await avatarCacheRepository.patchAvatarTags(
            normalizedAvatarId,
            previousEntries,
            nextEntries
        );
    }

    return nextEntries;
}

async function saveAvatar({ avatarId, params = {} }: SaveAvatarInput) {
    const normalizedAvatarId =
        typeof avatarId === 'string' ? avatarId.trim() : '';
    if (!normalizedAvatarId) {
        throw new Error('MyAvatarRepository.saveAvatar requires an avatar id.');
    }

    const response = unwrapVrchatAvatarResponse<AvatarRecord>(
        await commands.appVrchatAvatarSave({
            avatarId: normalizedAvatarId,
            params: {
                id: normalizedAvatarId,
                ...params
            }
        }),
        `avatars/${encodeURIComponent(normalizedAvatarId)}`
    );

    return response.json;
}

async function createImpostor({ avatarId }: AvatarIdInput = {}) {
    const normalizedAvatarId =
        typeof avatarId === 'string' ? avatarId.trim() : '';
    if (!normalizedAvatarId) {
        throw new Error(
            'MyAvatarRepository.createImpostor requires an avatar id.'
        );
    }

    const response = unwrapVrchatAvatarResponse(
        await commands.appVrchatAvatarImpostorCreate({
            avatarId: normalizedAvatarId
        }),
        `avatars/${encodeURIComponent(normalizedAvatarId)}/impostor/enqueue`
    );

    return response.json;
}

async function getAvailableAvatarStyles({
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

const myAvatarRepository = Object.freeze({
    getMyAvatarById,
    getMyAvatars,
    updateAvatarTags,
    saveAvatar,
    createImpostor,
    getAvailableAvatarStyles
});

export {
    getMyAvatarById,
    getMyAvatars,
    updateAvatarTags,
    saveAvatar,
    createImpostor,
    getAvailableAvatarStyles
};
export default myAvatarRepository;
