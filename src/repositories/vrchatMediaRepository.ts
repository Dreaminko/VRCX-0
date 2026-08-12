import {
    entityQueryPolicies,
    fetchCachedData,
    queryKeys
} from '@/lib/entityQueryCache';
import { commands } from '@/platform/tauri/bindings';
import type {
    MediaAssetKind,
    PrintFavoriteState
} from '@/platform/tauri/bindings';
import { normalizeString } from '@/shared/utils/string';
import { DEFAULT_VRCHAT_API_ENDPOINT } from '@/shared/vrchatEndpoint';

import { normalizePlatformError } from '../platform/tauri/errors';
import {
    isVrchatRequestError,
    type QueryParams,
    type QueryValue,
    type VrchatRequestResponse,
    unwrapVrchatResponse
} from './vrchatRequest';

type MediaApiRecord = Record<string, unknown>;
type MediaApiParams = QueryParams;

export type MediaFileVersion = Record<string, unknown> & {
    created_at?: string;
    file?: { url?: string } | null;
    status?: string;
    version?: number;
};

export type MediaFileRecord = MediaApiRecord & {
    animationStyle?: string;
    displayName?: string;
    extension?: string;
    frames?: number;
    framesOverTime?: number;
    id: string;
    maskTag?: string;
    loopStyle?: string;
    mimeType?: string;
    modifiedThumbnailFileName?: string;
    name?: string;
    ownerId?: string;
    tags?: string[];
    versions?: MediaFileVersion[];
};

type MediaPrintFiles = MediaApiRecord & {
    fileId?: string;
    image?: string;
};

export type MediaPrintRecord = MediaApiRecord & {
    authorId?: string;
    authorName?: string;
    createdAt?: string;
    files?: MediaPrintFiles;
    id: string;
    note?: string;
    ownerId?: string;
    timestamp?: string;
    worldId?: string;
    worldName?: string;
};

type InventoryAttribution = MediaApiRecord & {
    creator?: {
        customName?: string;
        userId?: string;
        [key: string]: unknown;
    } | null;
};

type InventoryAttribute = MediaApiRecord & {
    defaultValue?: unknown;
    validator?: MediaApiRecord;
};

export type InventoryAsset = MediaApiRecord & {
    fileId?: string;
    frameCount?: number;
    framesPerSecond?: number;
    loopCount?: number;
    totalDurationMs?: number;
    type?: string;
    url?: string;
};

export type InventoryItemMetadata = MediaApiRecord & {
    assets?: InventoryAsset[];
    gradientEnd?: string;
    gradientStart?: string;
    imageUrl?: string;
};

export type InventoryItemRecord = MediaApiRecord & {
    acquisition?: string;
    attribution?: InventoryAttribution | null;
    collections?: unknown[];
    created_at?: string;
    createdAt?: string;
    defaultAttributes?: Record<string, InventoryAttribute>;
    description?: string;
    equipSlot?: string;
    equipSlots?: string[];
    expiryDate?: string | null;
    flags?: string[];
    holderId?: string;
    id: string;
    imageUrl?: string;
    thumbnailUrl?: string;
    isArchived?: boolean;
    archived?: boolean;
    isSeen?: boolean;
    itemType?: string;
    type?: string;
    itemTypeLabel?: string;
    last_equipped?: Record<string, string> | null;
    metadata?: InventoryItemMetadata;
    name?: string;
    templateId?: string;
    item?: InventoryItemRecord | null;
    template?: InventoryItemRecord | null;
};

export type InventoryItemsResponse = {
    data: InventoryItemRecord[];
    totalCount: number;
};

export type InventoryItemsCollectResult = {
    items: InventoryItemRecord[];
    truncated: boolean;
};

const PROFILE_DECORATION_EQUIP_SLOTS = [
    'iconFrame',
    'profileEffect',
    'nameplateEffect'
] as const;

type ProfileDecorationEquipSlot =
    (typeof PROFILE_DECORATION_EQUIP_SLOTS)[number];

type ProfileDecorationEquipInput = {
    expectedUserId: unknown;
    inventoryId: unknown;
    equipSlot: unknown;
};

type ProfileDecorationUnequipInput = {
    expectedUserId: unknown;
    equipSlot: unknown;
};

interface MediaApiOptions {
    force?: boolean;
}

interface MediaUploadResponse {
    json: MediaApiRecord;
    params: MediaApiParams;
    status?: number;
}

interface LegacyImageUploadOptions {
    avatarId?: unknown;
    worldId?: unknown;
    imageUrl?: string;
    base64File: string;
}

interface MediaAssetUploadOptions {
    assetKind: MediaAssetKind;
    cropWhiteBorder?: boolean;
    params?: MediaApiParams;
}

interface MediaCommandOptions {
    params?: MediaApiParams;
    extra?: MediaApiRecord;
    fallbackMessage?: string;
    path?: string;
}

function normalizeParams(params: unknown = {}): MediaApiParams {
    if (!params || typeof params !== 'object') {
        return {};
    }
    return { ...(params as Record<string, QueryValue | QueryValue[]>) };
}

function unwrapMediaResponse(
    response: { status: number; data: unknown },
    options?: MediaCommandOptions
): VrchatRequestResponse<MediaApiRecord>;
function unwrapMediaResponse<TJson>(
    response: { status: number; data: unknown },
    options?: MediaCommandOptions
): VrchatRequestResponse<TJson>;
function unwrapMediaResponse<TJson = MediaApiRecord>(
    response: { status: number; data: unknown },
    {
        params = {},
        extra = {},
        fallbackMessage = 'Media request failed',
        path = 'media'
    }: MediaCommandOptions = {}
): VrchatRequestResponse<TJson> {
    return {
        ...unwrapVrchatResponse<TJson>(response, path, { fallbackMessage }),
        params,
        ...extra,
        status: response.status
    };
}

async function executeMediaCommand(
    command: () => Promise<{ status: number; data: unknown }>,
    options?: MediaCommandOptions
): Promise<VrchatRequestResponse<MediaApiRecord>>;
async function executeMediaCommand<TJson>(
    command: () => Promise<{ status: number; data: unknown }>,
    options?: MediaCommandOptions
): Promise<VrchatRequestResponse<TJson>>;
async function executeMediaCommand<TJson = MediaApiRecord>(
    command: () => Promise<{ status: number; data: unknown }>,
    options: MediaCommandOptions = {}
): Promise<VrchatRequestResponse<TJson>> {
    try {
        return unwrapMediaResponse<TJson>(await command(), options);
    } catch (error) {
        if (isVrchatRequestError(error)) {
            throw error;
        }
        throw normalizePlatformError(
            error,
            options.fallbackMessage ?? 'Media request failed'
        );
    }
}

async function getFiles(
    params: MediaApiParams = {}
): Promise<VrchatRequestResponse<MediaFileRecord[]>> {
    const normalizedParams = normalizeParams(params);
    return executeMediaCommand<MediaFileRecord[]>(
        () =>
            commands.appVrchatMediaFilesGet({
                params: normalizedParams
            }),
        {
            params: normalizedParams
        }
    );
}

async function getFileList(params: MediaApiParams = {}) {
    return getFiles(params);
}

async function deleteFile(fileId: unknown) {
    const normalizedFileId =
        typeof fileId === 'string'
            ? fileId.trim()
            : String(fileId ?? '').trim();
    if (!normalizedFileId) {
        throw new Error('MediaRepository.deleteFile requires a file id.');
    }

    return executeMediaCommand(
        () =>
            commands.appVrchatMediaFileDelete({
                fileId: normalizedFileId
            }),
        {
            extra: {
                fileId: normalizedFileId
            }
        }
    );
}

async function uploadGalleryImage(imageData: string) {
    const params: MediaApiParams = {
        tag: 'gallery'
    };
    return executeMediaCommand(
        () =>
            commands.appVrchatMediaGalleryImageUpload({
                imageData
            }),
        {
            params
        }
    );
}

async function uploadAvatarGalleryImage(
    imageData: string,
    avatarId: QueryValue
) {
    const params: MediaApiParams = {
        tag: 'avatargallery',
        galleryId: avatarId
    };
    return executeMediaCommand(
        () =>
            commands.appVrchatMediaAvatarGalleryImageUpload({
                imageData,
                avatarId
            }),
        {
            params
        }
    );
}

async function uploadVrcPlusIcon(imageData: string) {
    const params: MediaApiParams = {
        tag: 'icon'
    };
    return executeMediaCommand(
        () =>
            commands.appVrchatMediaVrcPlusIconUpload({
                imageData
            }),
        {
            params
        }
    );
}

async function uploadEmoji(imageData: string, params: MediaApiParams = {}) {
    const normalizedParams = normalizeParams(params);
    return executeMediaCommand(
        () =>
            commands.appVrchatMediaEmojiUpload({
                imageData,
                params: normalizedParams
            }),
        {
            params: normalizedParams
        }
    );
}

async function uploadSticker(imageData: string) {
    const params: MediaApiParams = {
        tag: 'sticker',
        maskTag: 'square'
    };
    return executeMediaCommand(
        () =>
            commands.appVrchatMediaStickerUpload({
                imageData
            }),
        {
            params
        }
    );
}

async function uploadPrint(
    imageData: string,
    {
        cropWhiteBorder = true,
        params = {}
    }: {
        cropWhiteBorder?: boolean;
        params?: MediaApiParams;
    } = {}
): Promise<MediaUploadResponse> {
    const normalizedParams = normalizeParams(params);
    const response = await executeMediaCommand(
        () =>
            commands.appVrchatMediaPrintUpload({
                imageData,
                cropWhiteBorder: Boolean(cropWhiteBorder),
                params: normalizedParams
            }),
        {
            params: normalizedParams,
            fallbackMessage: 'Print upload failed'
        }
    );
    return {
        ...response,
        params: response.params ?? normalizedParams
    };
}

async function uploadAssetImage(
    imageData: string,
    { assetKind, cropWhiteBorder = false, params = {} }: MediaAssetUploadOptions
): Promise<MediaUploadResponse> {
    const normalizedParams = normalizeParams(params);
    const response = await executeMediaCommand(
        () =>
            commands.appVrchatMediaAssetUpload({
                assetKind,
                imageData,
                cropWhiteBorder: Boolean(cropWhiteBorder),
                params: normalizedParams
            }),
        {
            params: normalizedParams,
            fallbackMessage: 'Media asset upload failed'
        }
    );
    return {
        ...response,
        params: response.params ?? normalizedParams
    };
}

async function getPrints({
    userId,
    n = 100
}: { userId?: unknown; n?: number } = {}): Promise<
    VrchatRequestResponse<MediaPrintRecord[]>
> {
    const normalizedUserId =
        typeof userId === 'string'
            ? userId.trim()
            : String(userId ?? '').trim();
    if (!normalizedUserId) {
        throw new Error('MediaRepository.getPrints requires a user id.');
    }

    return executeMediaCommand<MediaPrintRecord[]>(
        () =>
            commands.appVrchatMediaPrintsGet({
                userId: normalizedUserId,
                n
            }),
        {
            params: {
                n
            },
            extra: {
                userId: normalizedUserId
            }
        }
    );
}

async function getPrint(printId: unknown) {
    const normalizedPrintId =
        typeof printId === 'string'
            ? printId.trim()
            : String(printId ?? '').trim();
    if (!normalizedPrintId) {
        throw new Error('MediaRepository.getPrint requires a print id.');
    }

    return executeMediaCommand(
        () =>
            commands.appVrchatMediaPrintGet({
                printId: normalizedPrintId
            }),
        {
            extra: {
                printId: normalizedPrintId
            }
        }
    );
}

async function deletePrint(printId: unknown) {
    const normalizedPrintId =
        typeof printId === 'string'
            ? printId.trim()
            : String(printId ?? '').trim();
    if (!normalizedPrintId) {
        throw new Error('MediaRepository.deletePrint requires a print id.');
    }

    return executeMediaCommand(
        () =>
            commands.appVrchatMediaPrintDelete({
                printId: normalizedPrintId
            }),
        {
            extra: {
                printId: normalizedPrintId
            }
        }
    );
}

async function getPrintFavorites(): Promise<PrintFavoriteState> {
    return commands.appVrchatPrintsFavoritesList();
}

async function setPrintFavorite(
    printId: unknown,
    favoriteValue: unknown
): Promise<PrintFavoriteState> {
    const normalizedPrintId =
        typeof printId === 'string'
            ? printId.trim()
            : String(printId ?? '').trim();
    if (!normalizedPrintId) {
        throw new Error(
            'MediaRepository.setPrintFavorite requires a print id.'
        );
    }

    return commands.appVrchatPrintsFavoriteSet({
        printId: normalizedPrintId,
        favorite: favoriteValue === true
    });
}

async function getInventoryItems(
    params: MediaApiParams = {}
): Promise<VrchatRequestResponse<InventoryItemsResponse>> {
    const normalizedParams = normalizeParams(params);
    return executeMediaCommand<InventoryItemsResponse>(
        () =>
            commands.appVrchatMediaInventoryItemsGet({
                params: normalizedParams
            }),
        {
            params: normalizedParams
        }
    );
}

async function collectInventoryItems(
    params: MediaApiParams = {}
): Promise<InventoryItemsCollectResult> {
    const normalizedParams = normalizeParams(params);
    try {
        const result = await commands.appVrchatMediaInventoryItemsCollect({
            params: normalizedParams
        });
        const items = (result.items ?? []).flatMap((value) => {
            if (!value || typeof value !== 'object' || Array.isArray(value)) {
                return [];
            }
            const record: Record<string, unknown> = Object.fromEntries(
                Object.entries(value)
            );
            return typeof record.id === 'string'
                ? [{ ...record, id: record.id }]
                : [];
        });
        return {
            items,
            truncated: Boolean(result.truncated)
        };
    } catch (error) {
        throw normalizePlatformError(error, 'Media request failed');
    }
}

async function getInventoryTemplate(
    inventoryTemplateId: unknown
): Promise<VrchatRequestResponse<InventoryItemRecord>> {
    const normalizedInventoryTemplateId =
        typeof inventoryTemplateId === 'string'
            ? inventoryTemplateId.trim()
            : String(inventoryTemplateId ?? '').trim();
    if (!normalizedInventoryTemplateId) {
        throw new Error(
            'MediaRepository.getInventoryTemplate requires an inventory template id.'
        );
    }

    return fetchCachedData({
        queryKey: queryKeys.inventoryTemplate(
            normalizedInventoryTemplateId,
            DEFAULT_VRCHAT_API_ENDPOINT
        ),
        policy: entityQueryPolicies.inventoryTemplate,
        queryFn: () =>
            executeMediaCommand<InventoryItemRecord>(
                () =>
                    commands.appVrchatMediaInventoryTemplateGet({
                        inventoryTemplateId: normalizedInventoryTemplateId
                    }),
                {
                    extra: {
                        inventoryTemplateId: normalizedInventoryTemplateId
                    }
                }
            )
    });
}

function normalizeProfileDecorationEquipSlot(
    value: unknown
): ProfileDecorationEquipSlot | null {
    const normalizedValue = normalizeString(value);
    return (
        PROFILE_DECORATION_EQUIP_SLOTS.find(
            (slot) => slot === normalizedValue
        ) ?? null
    );
}

async function equipProfileDecoration({
    expectedUserId,
    inventoryId,
    equipSlot
}: ProfileDecorationEquipInput) {
    const normalizedExpectedUserId = normalizeString(expectedUserId);
    const normalizedInventoryId = normalizeString(inventoryId);
    const normalizedEquipSlot = normalizeProfileDecorationEquipSlot(equipSlot);
    if (!normalizedExpectedUserId) {
        throw new Error(
            'MediaRepository.equipProfileDecoration requires a user id.'
        );
    }
    if (!normalizedInventoryId) {
        throw new Error(
            'MediaRepository.equipProfileDecoration requires an inventory id.'
        );
    }
    if (!normalizedEquipSlot) {
        throw new Error(
            'MediaRepository.equipProfileDecoration requires a profile decoration slot.'
        );
    }

    return executeMediaCommand(
        () =>
            commands.appVrchatMediaProfileDecorationEquip({
                inventoryId: normalizedInventoryId,
                equipSlot: normalizedEquipSlot
            }),
        {
            extra: {
                inventoryId: normalizedInventoryId,
                equipSlot: normalizedEquipSlot
            }
        }
    );
}

async function unequipProfileDecoration({
    expectedUserId,
    equipSlot
}: ProfileDecorationUnequipInput) {
    const normalizedExpectedUserId = normalizeString(expectedUserId);
    const normalizedEquipSlot = normalizeProfileDecorationEquipSlot(equipSlot);
    if (!normalizedExpectedUserId) {
        throw new Error(
            'MediaRepository.unequipProfileDecoration requires a user id.'
        );
    }
    if (!normalizedEquipSlot) {
        throw new Error(
            'MediaRepository.unequipProfileDecoration requires a profile decoration slot.'
        );
    }

    return executeMediaCommand<string>(
        () =>
            commands.appVrchatMediaProfileDecorationUnequip({
                equipSlot: normalizedEquipSlot
            }),
        {
            extra: {
                equipSlot: normalizedEquipSlot
            }
        }
    );
}

async function getUserInventoryItem(
    { inventoryId, userId }: { inventoryId?: unknown; userId?: unknown } = {},
    options: MediaApiOptions = {}
) {
    const normalizedInventoryId =
        typeof inventoryId === 'string'
            ? inventoryId.trim()
            : String(inventoryId ?? '').trim();
    const normalizedUserId =
        typeof userId === 'string'
            ? userId.trim()
            : String(userId ?? '').trim();
    if (!normalizedInventoryId || !normalizedUserId) {
        throw new Error(
            'MediaRepository.getUserInventoryItem requires inventory and user ids.'
        );
    }

    return fetchCachedData({
        queryKey: queryKeys.userInventoryItem(
            {
                inventoryId: normalizedInventoryId,
                userId: normalizedUserId
            },
            DEFAULT_VRCHAT_API_ENDPOINT
        ),
        policy: entityQueryPolicies.inventoryCollection,
        force: Boolean(options.force),
        queryFn: () =>
            executeMediaCommand(
                () =>
                    commands.appVrchatMediaUserInventoryItemGet({
                        userId: normalizedUserId,
                        inventoryId: normalizedInventoryId
                    }),
                {
                    extra: {
                        inventoryId: normalizedInventoryId,
                        userId: normalizedUserId
                    }
                }
            )
    });
}

async function updateInventoryItem(
    inventoryId: unknown,
    params: MediaApiParams = {}
) {
    const normalizedInventoryId =
        typeof inventoryId === 'string'
            ? inventoryId.trim()
            : String(inventoryId ?? '').trim();
    if (!normalizedInventoryId) {
        throw new Error(
            'MediaRepository.updateInventoryItem requires an inventory id.'
        );
    }

    const normalizedParams = normalizeParams(params);
    return executeMediaCommand(
        () =>
            commands.appVrchatMediaInventoryItemUpdate({
                inventoryId: normalizedInventoryId,
                params: normalizedParams
            }),
        {
            params: normalizedParams
        }
    );
}

async function consumeInventoryBundle(inventoryId: unknown) {
    const normalizedInventoryId =
        typeof inventoryId === 'string'
            ? inventoryId.trim()
            : String(inventoryId ?? '').trim();
    if (!normalizedInventoryId) {
        throw new Error(
            'MediaRepository.consumeInventoryBundle requires an inventory id.'
        );
    }

    return executeMediaCommand(
        () =>
            commands.appVrchatMediaInventoryBundleConsume({
                inventoryId: normalizedInventoryId
            }),
        {
            params: {
                inventoryId: normalizedInventoryId
            }
        }
    );
}

async function redeemReward(code: unknown) {
    const normalizedCode =
        typeof code === 'string' ? code.trim() : String(code ?? '').trim();
    if (!normalizedCode) {
        throw new Error('MediaRepository.redeemReward requires a reward code.');
    }

    return executeMediaCommand(
        () =>
            commands.appVrchatMediaRewardRedeem({
                code: normalizedCode
            }),
        {
            params: {
                code: normalizedCode
            }
        }
    );
}

async function uploadAvatarImageLegacy({
    avatarId,
    imageUrl = '',
    base64File
}: LegacyImageUploadOptions) {
    const normalizedAvatarId =
        typeof avatarId === 'string'
            ? avatarId.trim()
            : String(avatarId ?? '').trim();
    if (!normalizedAvatarId) {
        throw new Error(
            'MediaRepository.uploadAvatarImageLegacy requires an avatar id.'
        );
    }

    const response = await executeMediaCommand(
        () =>
            commands.appVrchatMediaAvatarImageUploadLegacy({
                entityId: normalizedAvatarId,
                imageUrl,
                base64File,
                fileSizeInBytes: null
            }),
        {
            fallbackMessage: 'Avatar image upload failed'
        }
    );

    return {
        avatar: response.json?.avatar,
        imageUrl: response.json?.imageUrl,
        fileId: response.json?.fileId,
        fileVersion: response.json?.fileVersion
    };
}

async function uploadWorldImageLegacy({
    worldId,
    imageUrl = '',
    base64File
}: LegacyImageUploadOptions) {
    const normalizedWorldId =
        typeof worldId === 'string'
            ? worldId.trim()
            : String(worldId ?? '').trim();
    if (!normalizedWorldId) {
        throw new Error(
            'MediaRepository.uploadWorldImageLegacy requires a world id.'
        );
    }

    const response = await executeMediaCommand(
        () =>
            commands.appVrchatMediaWorldImageUploadLegacy({
                entityId: normalizedWorldId,
                imageUrl,
                base64File,
                fileSizeInBytes: null
            }),
        {
            fallbackMessage: 'World image upload failed'
        }
    );

    return {
        world: response.json?.world,
        imageUrl: response.json?.imageUrl,
        fileId: response.json?.fileId,
        fileVersion: response.json?.fileVersion
    };
}

const vrchatMediaRepository = Object.freeze({
    getFiles,
    getFileList,
    deleteFile,
    uploadGalleryImage,
    uploadAvatarGalleryImage,
    uploadVrcPlusIcon,
    uploadEmoji,
    uploadSticker,
    uploadPrint,
    uploadAssetImage,
    getPrints,
    getPrint,
    deletePrint,
    getPrintFavorites,
    setPrintFavorite,
    getInventoryItems,
    collectInventoryItems,
    getInventoryTemplate,
    equipProfileDecoration,
    unequipProfileDecoration,
    getUserInventoryItem,
    updateInventoryItem,
    consumeInventoryBundle,
    redeemReward,
    uploadAvatarImageLegacy,
    uploadWorldImageLegacy
});

export {
    getFiles,
    getFileList,
    deleteFile,
    uploadGalleryImage,
    uploadAvatarGalleryImage,
    uploadVrcPlusIcon,
    uploadEmoji,
    uploadSticker,
    uploadPrint,
    uploadAssetImage,
    getPrints,
    getPrint,
    deletePrint,
    getPrintFavorites,
    setPrintFavorite,
    getInventoryItems,
    collectInventoryItems,
    getInventoryTemplate,
    equipProfileDecoration,
    unequipProfileDecoration,
    getUserInventoryItem,
    updateInventoryItem,
    consumeInventoryBundle,
    redeemReward,
    uploadAvatarImageLegacy,
    uploadWorldImageLegacy
};

export default vrchatMediaRepository;
