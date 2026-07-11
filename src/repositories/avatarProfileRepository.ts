import type {
    AvatarLocalTag,
    AvatarProfileRecord,
    EntityRecord,
    UnityPackageRecord
} from '@/domain/entities/profileEntities';
import {
    entityQueryPolicies,
    fetchCachedData,
    invalidateEntityQueries,
    queryKeys,
    setCachedQueryData
} from '@/lib/entityQueryCache';
import {
    commands,
    type HttpApiExecuteResponse,
    type VrchatAvatarEndpointInput,
    type VrchatAvatarFileInput,
    type VrchatAvatarIdInput as IpcVrchatAvatarIdInput,
    type VrchatAvatarImpostorCreateInput,
    type VrchatAvatarListByUserInput,
    type VrchatAvatarModerationInput as IpcVrchatAvatarModerationInput,
    type VrchatAvatarSaveInput
} from '@/platform/tauri/bindings';
import { storeAvatarImage } from '@/shared/utils/avatar';
import { extractFileId } from '@/shared/utils/fileUtils';
import { normalizeVrchatEndpointDomain } from '@/shared/vrchatEndpoint';

import avatarCacheRepository from './avatarCacheRepository';
import memoPersistenceRepository from './memoPersistenceRepository';
import {
    VRCHAT_API_DEFAULT_PAGE_SIZE,
    VRCHAT_PROFILE_MAX_PAGES
} from './paginationConstants';
import {
    createRequestError,
    notifyVrchatAuthFailure,
    parseJsonResponse,
    unwrapErrorMessage
} from './vrchatRequest';

type AvatarRecord = Record<string, unknown>;
type CachedAvatarImage = ReturnType<typeof storeAvatarImage>;
export type AvatarStyleRecord = AvatarRecord & {
    id?: string;
    name?: string;
    styleName?: string;
};
export type AvatarGalleryFile = AvatarRecord & {
    id?: string;
    fileId?: string;
    order?: number | string;
    url?: string;
    fileUrl?: string;
    imageUrl?: string;
    versions?: Array<
        AvatarRecord & {
            file?: AvatarRecord & { url?: string };
        }
    >;
};
type AvatarFileVersion = AvatarRecord & {
    created_at?: string;
    status?: string;
    version?: number;
};
type AvatarFileRecord = AvatarRecord & {
    extension?: string;
    id?: string;
    mimeType?: string;
    name?: string;
    ownerId?: string;
    tags?: string[];
    versions?: AvatarFileVersion[];
};
export type AvatarModerationRecord = AvatarRecord & {
    avatarModerationType?: string;
    created?: string | number;
    targetAvatarId?: string;
};
type AvatarModerationDeleteRecord = AvatarRecord & {
    OK?: string;
};

interface AvatarProfileExtras extends AvatarRecord {
    cachedAvatar?: unknown;
    localTags?: unknown[];
    timeSpent?: unknown;
    memo?: unknown;
}

interface AvatarListOptions {
    userId?: unknown;
    user?: string;
    endpoint?: string;
    n?: number;
    offset?: number;
    sort?: string;
    order?: string;
    releaseStatus?: string;
}

interface AvatarRequestOptions {
    endpoint?: string;
}

interface AvatarIdInput extends AvatarRequestOptions {
    avatarId?: unknown;
}

interface SaveAvatarInput extends AvatarIdInput {
    params?: Record<string, unknown>;
}

interface AvatarStylesInput extends AvatarRequestOptions {
    force?: boolean;
}

interface CollectPagesOptions {
    pageSize?: number;
    maxPages?: number;
}

interface AvatarProfileInput extends AvatarIdInput {
    force?: boolean;
    dialog?: boolean;
    allowLocalFallback?: boolean;
    currentUserId?: unknown;
}

interface AvatarModerationInput extends AvatarIdInput {
    type?: unknown;
}

const cachedAvatarNames = new Map<string, CachedAvatarImage>();

function normalizeEntityId(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeString(value: unknown): string {
    return typeof value === 'string' ? value.trim() : '';
}

function normalizeTimestamp(value: unknown): string {
    return typeof value === 'string' ? value : '';
}

function avatarIdInput(
    avatarId: string,
    endpoint: string
): IpcVrchatAvatarIdInput {
    return { avatarId, endpoint };
}

function avatarEndpointInput(endpoint: string): VrchatAvatarEndpointInput {
    return { endpoint };
}

function normalizeMemoString(value: unknown): string {
    return typeof value === 'string' ? value : '';
}

function normalizeArray(values: unknown): string[] {
    if (!Array.isArray(values)) {
        return [];
    }

    return values
        .map((value) =>
            typeof value === 'string'
                ? value.trim()
                : String(value ?? '').trim()
        )
        .filter(Boolean);
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function unwrapVrchatAvatarResponse<TJson = unknown>(
    response: HttpApiExecuteResponse,
    path: string
) {
    const json = parseJsonResponse(response.data);
    if (response.status >= 400 || (isRecord(json) && 'error' in json)) {
        const requestError = createRequestError(
            unwrapErrorMessage(json, response.status, {
                fallbackMessage: 'VRChat avatar request failed'
            }),
            response.status,
            path,
            json
        );
        notifyVrchatAuthFailure(requestError);
        throw requestError;
    }

    return {
        json: json as TJson,
        status: response.status,
        raw: response.raw
    };
}

function normalizeLocalTags(values: unknown): AvatarLocalTag[] {
    if (!Array.isArray(values)) {
        return [];
    }

    return values
        .map((entry) => {
            const source = isRecord(entry) ? entry : {};
            return {
                tag: normalizeString(source.tag),
                color: normalizeString(source.color) || null
            };
        })
        .filter((entry) => entry.tag);
}

function normalizeUnityPackages(values: unknown): UnityPackageRecord[] {
    if (!Array.isArray(values)) {
        return [];
    }

    return values.filter((value): value is EntityRecord =>
        Boolean(value && typeof value === 'object')
    );
}

function parseInteger(value: unknown): number {
    const parsed = Number.parseInt(String(value), 10);
    return Number.isFinite(parsed) ? parsed : 0;
}

function normalizeFileResponse(json: unknown): {
    versions: AvatarFileVersion[];
    name?: string;
    ownerId?: string;
} {
    if (isRecord(json) && Array.isArray(json.versions)) {
        return {
            versions: json.versions.filter(isRecord),
            name: typeof json.name === 'string' ? json.name : '',
            ownerId: typeof json.ownerId === 'string' ? json.ownerId : ''
        };
    }

    return { versions: [], name: '', ownerId: '' };
}

function normalizeAvatarProfile(
    avatar: unknown,
    extras: AvatarProfileExtras = {}
): AvatarProfileRecord {
    const source = isRecord(avatar) ? avatar : {};
    return {
        ...source,
        id: normalizeEntityId(source.id),
        name: normalizeString(source.name),
        description: normalizeString(source.description),
        authorId: normalizeEntityId(source.authorId ?? source.author_id),
        authorName:
            normalizeEntityId(source.authorName ?? source.author_name) ||
            normalizeEntityId(source.authorId ?? source.author_id) ||
            'Unknown author',
        releaseStatus:
            normalizeEntityId(source.releaseStatus ?? source.release_status) ||
            'unknown',
        thumbnailImageUrl: normalizeString(
            source.thumbnailImageUrl ?? source.thumbnail_image_url
        ),
        imageUrl: normalizeString(source.imageUrl ?? source.image_url),
        created_at: normalizeTimestamp(source.created_at ?? source.createdAt),
        updated_at: normalizeTimestamp(source.updated_at ?? source.updatedAt),
        version: parseInteger(source.version),
        tags: normalizeArray(source.tags),
        unityPackages: normalizeUnityPackages(source.unityPackages),
        $tags: normalizeLocalTags(extras.localTags ?? source.$tags),
        $timeSpent: Math.max(
            0,
            parseInteger(extras.timeSpent ?? source.$timeSpent)
        ),
        $memo: normalizeMemoString(extras.memo ?? source.$memo),
        $isCached: Boolean(extras.cachedAvatar)
    };
}

async function collectPages<T>(
    fetchPage: (page: { n: number; offset: number }) => Promise<T[]>,
    {
        pageSize = VRCHAT_API_DEFAULT_PAGE_SIZE,
        maxPages = VRCHAT_PROFILE_MAX_PAGES
    }: CollectPagesOptions = {}
): Promise<T[]> {
    const rows: T[] = [];

    for (let page = 0; page < maxPages; page += 1) {
        const nextRows = await fetchPage({
            n: pageSize,
            offset: page * pageSize
        });
        rows.push(...nextRows);

        if (nextRows.length < pageSize) {
            break;
        }
    }

    return rows;
}

function normalize(
    avatar: unknown,
    extras: AvatarProfileExtras = {}
): AvatarProfileRecord {
    return normalizeAvatarProfile(avatar, extras);
}

function clearAvatarNameCache() {
    const size = cachedAvatarNames.size;
    cachedAvatarNames.clear();
    return size;
}

function getAvatarNameCacheSize() {
    return cachedAvatarNames.size;
}

async function getLocalSnapshot(
    avatarId: unknown,
    currentUserId: unknown = ''
): Promise<AvatarProfileExtras> {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        return {
            cachedAvatar: null,
            localTags: [],
            timeSpent: 0,
            memo: ''
        };
    }

    const [cachedAvatar, localTags, timeSpentEntry, memoEntry] =
        await Promise.all([
            avatarCacheRepository
                .getCachedAvatarById(normalizedAvatarId)
                .catch(
                    (): Awaited<
                        ReturnType<
                            typeof avatarCacheRepository.getCachedAvatarById
                        >
                    > | null => null
                ),
            avatarCacheRepository
                .getAvatarTags(normalizedAvatarId)
                .catch(
                    (): Awaited<
                        ReturnType<typeof avatarCacheRepository.getAvatarTags>
                    > => []
                ),
            currentUserId
                ? avatarCacheRepository
                      .getAvatarTimeSpent(currentUserId, normalizedAvatarId)
                      .catch(
                          (): Awaited<
                              ReturnType<
                                  typeof avatarCacheRepository.getAvatarTimeSpent
                              >
                          > | null => null
                      )
                : Promise.resolve(null),
            memoPersistenceRepository
                .getAvatarMemo(normalizedAvatarId)
                .catch(
                    (): Awaited<
                        ReturnType<
                            typeof memoPersistenceRepository.getAvatarMemo
                        >
                    > | null => null
                )
        ]);

    return {
        cachedAvatar: cachedAvatar || null,
        localTags: normalizeLocalTags(localTags),
        timeSpent: parseInteger(timeSpentEntry?.timeSpent),
        memo: normalizeString(memoEntry?.memo)
    };
}

async function getAvatarProfile({
    avatarId,
    endpoint = '',
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

    const localSnapshotPromise = getLocalSnapshot(
        normalizedAvatarId,
        currentUserId
    );

    try {
        const [json, localSnapshot] = await Promise.all([
            fetchCachedData({
                queryKey: queryKeys.avatar(normalizedAvatarId, endpoint),
                policy: dialog
                    ? entityQueryPolicies.avatarDialog
                    : entityQueryPolicies.avatar,
                force,
                queryFn: async () => {
                    const response = unwrapVrchatAvatarResponse<AvatarRecord>(
                        await commands.appVrchatAvatarGet(
                            avatarIdInput(normalizedAvatarId, endpoint)
                        ),
                        `avatars/${encodeURIComponent(normalizedAvatarId)}`
                    );
                    return response.json;
                }
            }),
            localSnapshotPromise
        ]);

        return normalize(json, localSnapshot);
    } catch (error) {
        const localSnapshot = await localSnapshotPromise;
        if (allowLocalFallback && localSnapshot.cachedAvatar) {
            return normalize(localSnapshot.cachedAvatar, localSnapshot);
        }

        throw error;
    }
}

async function getAvatarGallery({
    avatarId,
    endpoint = '',
    force = false
}: {
    avatarId?: unknown;
    endpoint?: string;
    force?: boolean;
}): Promise<AvatarGalleryFile[]> {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.getAvatarGallery requires an avatar id.'
        );
    }

    const rows = await fetchCachedData({
        queryKey: queryKeys.avatarGallery(normalizedAvatarId, endpoint),
        policy: entityQueryPolicies.avatarGallery,
        force,
        queryFn: async () => {
            const response = unwrapVrchatAvatarResponse(
                await commands.appVrchatAvatarGalleryGet(
                    avatarIdInput(normalizedAvatarId, endpoint)
                ),
                'files'
            );
            const rows = Array.isArray(response.json)
                ? response.json
                : isRecord(response.json) && Array.isArray(response.json.files)
                  ? response.json.files
                  : [];
            return rows.filter(isRecord);
        }
    });
    return rows.slice().sort((a, b) => {
        if (!a?.order && !b?.order) {
            return 0;
        }
        return (Number(a?.order) || 0) - (Number(b?.order) || 0);
    });
}

async function getAvatarsByUser({
    userId,
    user = '',
    endpoint = '',
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
        endpoint,
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

async function getAllAvatarsByUser({
    userId,
    user = '',
    endpoint = '',
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
            endpoint,
            n,
            offset,
            sort,
            order,
            releaseStatus
        })
    );
}

async function selectAvatar({ avatarId, endpoint = '' }: AvatarIdInput) {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.selectAvatar requires an avatar id.'
        );
    }

    const response = unwrapVrchatAvatarResponse<AvatarRecord>(
        await commands.appVrchatAvatarSelect(
            avatarIdInput(normalizedAvatarId, endpoint)
        ),
        `avatars/${encodeURIComponent(normalizedAvatarId)}/select`
    );
    if (response.json && typeof response.json === 'object') {
        setCachedQueryData(
            queryKeys.avatar(normalizedAvatarId, endpoint),
            response.json
        );
    }
    return response;
}

async function selectFallbackAvatar({
    avatarId,
    endpoint = ''
}: AvatarIdInput) {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.selectFallbackAvatar requires an avatar id.'
        );
    }

    const response = unwrapVrchatAvatarResponse<AvatarRecord>(
        await commands.appVrchatAvatarSelectFallback(
            avatarIdInput(normalizedAvatarId, endpoint)
        ),
        `avatars/${encodeURIComponent(normalizedAvatarId)}/selectfallback`
    );
    if (response.json && typeof response.json === 'object') {
        setCachedQueryData(
            queryKeys.avatar(normalizedAvatarId, endpoint),
            response.json
        );
    }
    return response;
}

async function saveAvatar({
    avatarId,
    params = {},
    endpoint = ''
}: SaveAvatarInput) {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.saveAvatar requires an avatar id.'
        );
    }

    const input = {
        avatarId: normalizedAvatarId,
        endpoint,
        params
    } satisfies VrchatAvatarSaveInput;
    const response = unwrapVrchatAvatarResponse<AvatarRecord>(
        await commands.appVrchatAvatarSave(input),
        `avatars/${encodeURIComponent(normalizedAvatarId)}`
    );
    if (response.json && typeof response.json === 'object') {
        setCachedQueryData(
            queryKeys.avatar(normalizedAvatarId, endpoint),
            response.json
        );
    }
    return response;
}

async function getAvatarStyles({
    endpoint = '',
    force = false
}: AvatarStylesInput = {}): Promise<AvatarStyleRecord[]> {
    return fetchCachedData({
        queryKey: queryKeys.avatarStyles(endpoint),
        policy: entityQueryPolicies.avatarStyles,
        force,
        queryFn: async () => {
            const response = unwrapVrchatAvatarResponse<AvatarStyleRecord[]>(
                await commands.appVrchatAvatarStylesGet(
                    avatarEndpointInput(endpoint)
                ),
                'avatarStyles'
            );
            return Array.isArray(response.json) ? response.json : [];
        }
    });
}

async function deleteAvatar({ avatarId, endpoint = '' }: AvatarIdInput) {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.deleteAvatar requires an avatar id.'
        );
    }

    const response = unwrapVrchatAvatarResponse<AvatarRecord>(
        await commands.appVrchatAvatarDelete(
            avatarIdInput(normalizedAvatarId, endpoint)
        ),
        `avatars/${encodeURIComponent(normalizedAvatarId)}`
    );
    await Promise.allSettled([
        invalidateEntityQueries(queryKeys.avatar(normalizedAvatarId, endpoint)),
        invalidateEntityQueries(
            queryKeys.avatarGallery(normalizedAvatarId, endpoint)
        )
    ]);
    return response;
}

async function createImposter({ avatarId, endpoint = '' }: AvatarIdInput) {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.createImposter requires an avatar id.'
        );
    }

    const input = {
        avatarId: normalizedAvatarId,
        endpoint,
        emptyBody: true
    } satisfies VrchatAvatarImpostorCreateInput;
    return unwrapVrchatAvatarResponse<AvatarRecord>(
        await commands.appVrchatAvatarImpostorCreate(input),
        `avatars/${encodeURIComponent(normalizedAvatarId)}/impostor/enqueue`
    );
}

async function deleteImposter({ avatarId, endpoint = '' }: AvatarIdInput) {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.deleteImposter requires an avatar id.'
        );
    }

    return unwrapVrchatAvatarResponse<AvatarRecord>(
        await commands.appVrchatAvatarImpostorDelete(
            avatarIdInput(normalizedAvatarId, endpoint)
        ),
        `avatars/${encodeURIComponent(normalizedAvatarId)}/impostor`
    );
}

async function getAvatarModerations({
    endpoint = ''
}: AvatarRequestOptions = {}) {
    const response = unwrapVrchatAvatarResponse(
        await commands.appVrchatAvatarModerationsGet(
            avatarEndpointInput(endpoint)
        ),
        'auth/user/avatarmoderations'
    );
    return {
        ...response,
        json: Array.isArray(response.json) ? response.json.filter(isRecord) : []
    };
}

async function sendAvatarModeration({
    avatarId,
    type = 'block',
    endpoint = ''
}: AvatarModerationInput) {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    const normalizedType = normalizeString(type) || 'block';
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.sendAvatarModeration requires an avatar id.'
        );
    }

    const input = {
        avatarId: normalizedAvatarId,
        type: normalizedType,
        endpoint
    } satisfies IpcVrchatAvatarModerationInput;
    return unwrapVrchatAvatarResponse<AvatarModerationRecord>(
        await commands.appVrchatAvatarModerationSend(input),
        'auth/user/avatarmoderations'
    );
}

async function deleteAvatarModeration({
    avatarId,
    type = 'block',
    endpoint = ''
}: AvatarModerationInput) {
    const normalizedAvatarId = normalizeEntityId(avatarId);
    const normalizedType = normalizeString(type) || 'block';
    if (!normalizedAvatarId) {
        throw new Error(
            'AvatarProfileRepository.deleteAvatarModeration requires an avatar id.'
        );
    }

    const input = {
        avatarId: normalizedAvatarId,
        type: normalizedType,
        endpoint
    } satisfies IpcVrchatAvatarModerationInput;
    return unwrapVrchatAvatarResponse<AvatarModerationDeleteRecord>(
        await commands.appVrchatAvatarModerationDelete(input),
        'auth/user/avatarmoderations'
    );
}

async function getAvatarNameFromImageUrl(
    imageUrl: unknown,
    { endpoint = '' }: AvatarRequestOptions = {}
) {
    const fileId = extractFileId(String(imageUrl || ''));
    if (!fileId) {
        return {
            ownerId: '',
            avatarName: '-'
        };
    }

    const cacheKey = `${normalizeVrchatEndpointDomain(endpoint)}\u0000${fileId}`;
    if (cachedAvatarNames.has(cacheKey)) {
        return cachedAvatarNames.get(cacheKey);
    }

    try {
        const response = await fetchCachedData({
            queryKey: queryKeys.file(fileId, endpoint),
            policy: entityQueryPolicies.fileObject,
            queryFn: async () => {
                return unwrapVrchatAvatarResponse<AvatarFileRecord>(
                    await commands.appVrchatAvatarFileGet({
                        fileId,
                        endpoint
                    } satisfies VrchatAvatarFileInput),
                    `file/${encodeURIComponent(fileId)}`
                );
            }
        });
        const nextInfo = storeAvatarImage(
            {
                json: normalizeFileResponse(response.json),
                params: { fileId }
            },
            new Map()
        );
        cachedAvatarNames.set(cacheKey, nextInfo);
        return nextInfo;
    } catch {
        return {
            ownerId: '',
            avatarName: '-'
        };
    }
}

const avatarProfileRepository = Object.freeze({
    normalize,
    clearAvatarNameCache,
    getAvatarNameCacheSize,
    getLocalSnapshot,
    getAvatarProfile,
    getAvatarGallery,
    getAvatarsByUser,
    getAllAvatarsByUser,
    selectAvatar,
    selectFallbackAvatar,
    saveAvatar,
    getAvatarStyles,
    deleteAvatar,
    createImposter,
    deleteImposter,
    getAvatarModerations,
    sendAvatarModeration,
    deleteAvatarModeration,
    getAvatarNameFromImageUrl
});

export {
    normalize,
    clearAvatarNameCache,
    getAvatarNameCacheSize,
    getLocalSnapshot,
    getAvatarProfile,
    getAvatarGallery,
    getAvatarsByUser,
    getAllAvatarsByUser,
    selectAvatar,
    selectFallbackAvatar,
    saveAvatar,
    getAvatarStyles,
    deleteAvatar,
    createImposter,
    deleteImposter,
    getAvatarModerations,
    sendAvatarModeration,
    deleteAvatarModeration,
    getAvatarNameFromImageUrl
};
export type {
    AvatarProfileRecord,
    AvatarStyleSelection
} from '@/domain/entities/profileEntities';
export default avatarProfileRepository;
