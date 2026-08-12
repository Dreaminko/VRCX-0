import type { ChangeEvent } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import mediaRepository, {
    type InventoryItemRecord,
    type MediaFileRecord
} from '@/repositories/mediaRepository';
import { VRCHAT_API_DEFAULT_PAGE_SIZE } from '@/repositories/paginationConstants';
import { refreshCurrentUser } from '@/services/backgroundMaintenanceSessionService';
import {
    IMAGE_UPLOAD_ACCEPT,
    readFileAsBase64,
    withUploadTimeout
} from '@/shared/utils/imageUpload';
import { useModalStore } from '@/state/modalStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import {
    CATEGORY_DEFINITIONS,
    INITIAL_INVENTORY_SUB_TABS,
    getInventoryGridDensityConfig,
    parseEmojiUploadSettings,
    readGridDensityPreference,
    resolveProfileDecorationMutation,
    sanitizeInventoryGridDensity,
    scopeKey,
    validateImageFile,
    writeGridDensityPreference,
    type InventoryCategory,
    type InventoryTabDefinition,
    type InventoryUploadTarget
} from './inventoryHelpers';

export { IMAGE_UPLOAD_ACCEPT };

type InventoryAuthTarget = {
    endpoint: string;
    userId: string;
    websocket: string;
};

export type InventoryRow = MediaFileRecord & Partial<InventoryItemRecord>;

const EMPTY_ROWS_BY_SCOPE: Record<string, InventoryRow[]> = Object.freeze({});
const EMPTY_LOADING_BY_SCOPE: Record<string, boolean> = Object.freeze({});

type InventoryUploadSettings = {
    animationStyle: string;
    fps: number;
    frames: number;
    isAnimated: boolean;
    loopPingPong: boolean;
};

type InventoryCropRequest = {
    aspectRatio: number;
    authTarget: InventoryAuthTarget;
    file: File;
    settings: InventoryUploadSettings;
    target: InventoryUploadTarget;
};

function inventoryAuthTargetKey(authTarget: InventoryAuthTarget) {
    return [authTarget.endpoint, authTarget.userId, authTarget.websocket].join(
        '\u0000'
    );
}

function getInventoryAuthTarget(): InventoryAuthTarget {
    const auth = useRuntimeStore.getState().auth;
    return {
        userId: auth.currentUserId || '',
        endpoint: auth.currentUserEndpoint || '',
        websocket: auth.currentUserWebsocket || ''
    };
}

function isCurrentInventoryAuthTarget(authTarget: InventoryAuthTarget) {
    const currentAuth = getInventoryAuthTarget();
    return (
        currentAuth.userId === authTarget.userId &&
        currentAuth.endpoint === authTarget.endpoint &&
        currentAuth.websocket === authTarget.websocket
    );
}

async function loadInventoryFileRows(
    definition: InventoryTabDefinition,
    authTarget: InventoryAuthTarget
) {
    const nextRows: InventoryRow[] = [];
    for (const tag of definition.fileTags || []) {
        const { json } = await mediaRepository.getFileList({
            n: VRCHAT_API_DEFAULT_PAGE_SIZE,
            tag
        });
        if (Array.isArray(json)) {
            nextRows.push(...json);
        }
    }
    const seen = new Set<string>();
    return nextRows
        .filter((row) => {
            if (!row?.id || seen.has(row.id)) {
                return false;
            }
            seen.add(row.id);
            return true;
        })
        .reverse()
        .filter(() => isCurrentInventoryAuthTarget(authTarget));
}

async function loadInventoryRows(definition: InventoryTabDefinition) {
    if (definition.source === 'empty') {
        return [];
    }
    const { items, truncated } = await mediaRepository.collectInventoryItems({
        order: 'newest',
        ...(definition.params || {})
    });
    if (truncated) {
        console.warn('Inventory listing truncated at the page limit.');
    }
    return items;
}

export function useInventoryPageState() {
    const { t } = useTranslation();
    const translateRef = useRef(t);
    useEffect(() => {
        translateRef.current = t;
    }, [t]);
    const uploadInputRef = useRef<HTMLInputElement | null>(null);
    const uploadTargetRef = useRef<InventoryUploadTarget | null>(null);
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const currentEndpoint = useRuntimeStore(
        (state) => state.auth.currentUserEndpoint
    );
    const currentUserWebsocket = useRuntimeStore(
        (state) => state.auth.currentUserWebsocket
    );
    const currentUserSnapshot = useRuntimeStore(
        (state) => state.auth.currentUserSnapshot
    );
    const confirm = useModalStore((state) => state.confirm);
    const prompt = useModalStore((state) => state.prompt);
    const openImagePreview = useModalStore((state) => state.openImagePreview);
    const [activeCategory, setActiveCategory] =
        useState<InventoryCategory>('emojis');
    const [activeSubTabs, setActiveSubTabs] = useState<
        Record<InventoryCategory, string>
    >({ ...INITIAL_INVENTORY_SUB_TABS });
    const [rowsByScope, setRowsByScope] = useState<
        Record<string, InventoryRow[]>
    >({});
    const [loadingByScope, setLoadingByScope] = useState<
        Record<string, boolean>
    >({});
    const currentAuthTargetKey = inventoryAuthTargetKey({
        userId: currentUserId || '',
        endpoint: currentEndpoint || '',
        websocket: currentUserWebsocket || ''
    });
    const [scopeStateAuthTargetKey, setScopeStateAuthTargetKey] =
        useState(currentAuthTargetKey);
    const [mutatingKey, setMutatingKey] = useState('');
    const profileDecorationMutationPendingRef = useRef(false);
    const [
        profileDecorationMutationPending,
        setProfileDecorationMutationPending
    ] = useState(false);
    const [uploadingTarget, setUploadingTarget] = useState('');
    const [cropRequest, setCropRequest] = useState<InventoryCropRequest | null>(
        null
    );
    const [emojiAnimFps, setEmojiAnimFps] = useState(15);
    const [emojiAnimFrameCount, setEmojiAnimFrameCount] = useState(4);
    const [emojiAnimType, setEmojiAnimType] = useState(false);
    const [emojiAnimationStyle, setEmojiAnimationStyle] = useState('Stop');
    const [emojiAnimLoopPingPong, setEmojiAnimLoopPingPong] = useState(false);
    const [gridDensity, setGridDensity] = useState(readGridDensityPreference);
    const gridDensityConfig = useMemo(
        () => getInventoryGridDensityConfig(gridDensity),
        [gridDensity]
    );
    const isVrcPlusSupporter = Boolean(
        currentUserSnapshot?.$isVRCPlus ||
        currentUserSnapshot?.tags?.includes?.('system_supporter') ||
        globalThis.$debug?.debugVrcPlus
    );

    const activeSubTab = activeSubTabs[activeCategory];
    const activeScopeKey = scopeKey(activeCategory, activeSubTab);

    function changeGridDensity(nextValue: unknown) {
        const nextDensity = sanitizeInventoryGridDensity(nextValue);
        setGridDensity(nextDensity);
        writeGridDensityPreference(nextDensity);
    }

    const setScopeLoading = useCallback((key: string, value: unknown) => {
        setLoadingByScope((current) => ({
            ...current,
            [key]: Boolean(value)
        }));
    }, []);

    const setScopeRows = useCallback((key: string, rows: InventoryRow[]) => {
        setRowsByScope((current) => ({
            ...current,
            [key]: rows
        }));
    }, []);

    const refreshScope = useCallback(
        async (
            category: InventoryCategory = activeCategory,
            tab: string = activeSubTab
        ) => {
            const definition = CATEGORY_DEFINITIONS[category].tabs.find(
                (entry) => entry.key === tab
            );
            if (!definition) {
                return;
            }
            const key = scopeKey(category, tab);
            const authTarget = getInventoryAuthTarget();
            setScopeLoading(key, true);
            try {
                const rows =
                    definition.source === 'file'
                        ? await loadInventoryFileRows(definition, authTarget)
                        : await loadInventoryRows(definition);
                if (isCurrentInventoryAuthTarget(authTarget)) {
                    setScopeRows(key, rows);
                }
            } catch (error) {
                if (isCurrentInventoryAuthTarget(authTarget)) {
                    toast.error(
                        error instanceof Error
                            ? error.message
                            : translateRef.current(
                                  'dialog.inventory.failed_to_load'
                              )
                    );
                }
            } finally {
                if (isCurrentInventoryAuthTarget(authTarget)) {
                    setScopeLoading(key, false);
                }
            }
        },
        [activeCategory, activeSubTab, setScopeLoading, setScopeRows]
    );

    useEffect(() => {
        setScopeStateAuthTargetKey(currentAuthTargetKey);
        setRowsByScope({});
        setLoadingByScope({});
        setMutatingKey('');
        profileDecorationMutationPendingRef.current = false;
        setProfileDecorationMutationPending(false);
    }, [currentAuthTargetKey]);

    useEffect(() => {
        if (currentUserId && currentAuthTargetKey) {
            refreshScope(activeCategory, activeSubTab);
        }
    }, [
        currentAuthTargetKey,
        currentUserId,
        activeCategory,
        activeSubTab,
        refreshScope
    ]);

    function beginUpload(target: InventoryUploadTarget) {
        if (!isVrcPlusSupporter) {
            toast.error(t('message.vrcplus.required'));
            return;
        }
        uploadTargetRef.current = target;
        uploadInputRef.current?.click();
    }

    function getEmojiUploadParams(settings: InventoryUploadSettings) {
        const params: Record<string, string | number> = {
            tag: settings.isAnimated ? 'emojianimated' : 'emoji',
            animationStyle: String(
                settings.animationStyle || 'Stop'
            ).toLowerCase(),
            maskTag: 'square'
        };
        if (settings.isAnimated) {
            params.frames = Math.min(
                64,
                Math.max(2, Number(settings.frames) || 4)
            );
            params.framesOverTime = Math.min(
                64,
                Math.max(1, Number(settings.fps) || 15)
            );
        }
        if (settings.loopPingPong) {
            params.loopStyle = 'pingpong';
        }
        return params;
    }

    function uploadAsset(
        target: unknown,
        base64Body: string,
        settings: InventoryUploadSettings
    ) {
        if (target === 'emojis') {
            return mediaRepository.uploadEmoji(
                base64Body,
                getEmojiUploadParams(settings)
            );
        }
        if (target === 'stickers') {
            return mediaRepository.uploadSticker(base64Body);
        }
        throw new Error(`Unsupported inventory upload target: ${target}`);
    }

    async function uploadSelectedFile(event: ChangeEvent<HTMLInputElement>) {
        const file = event.target.files?.[0] || null;
        event.target.value = '';
        if (!file) {
            return;
        }
        if (!isVrcPlusSupporter) {
            toast.error(t('message.vrcplus.required'));
            return;
        }
        if (!validateImageFile(file, t)) {
            return;
        }
        const target = uploadTargetRef.current;
        if (!target) {
            return;
        }
        const authTarget = getInventoryAuthTarget();
        const settings =
            target === 'emojis'
                ? parseEmojiUploadSettings(file.name, {
                      isAnimated: emojiAnimType,
                      animationStyle: emojiAnimationStyle,
                      fps: emojiAnimFps,
                      frames: emojiAnimFrameCount,
                      loopPingPong: emojiAnimLoopPingPong
                  })
                : {
                      isAnimated: false,
                      animationStyle: emojiAnimationStyle,
                      fps: emojiAnimFps,
                      frames: emojiAnimFrameCount,
                      loopPingPong: emojiAnimLoopPingPong
                  };
        if (target === 'emojis') {
            setEmojiAnimType(settings.isAnimated);
            setEmojiAnimationStyle(settings.animationStyle);
            setEmojiAnimFps(settings.fps);
            setEmojiAnimFrameCount(settings.frames);
            setEmojiAnimLoopPingPong(settings.loopPingPong);
        }
        setCropRequest({
            target,
            file,
            settings,
            authTarget,
            aspectRatio: 1
        });
    }

    async function confirmCroppedUpload(blob: Blob) {
        const request = cropRequest;
        if (
            !request ||
            !blob ||
            !isCurrentInventoryAuthTarget(request.authTarget)
        ) {
            return;
        }
        const { target, settings, authTarget } = request;
        setUploadingTarget(target ?? '');
        try {
            const base64Body = await readFileAsBase64(blob);
            if (!isCurrentInventoryAuthTarget(authTarget)) {
                return;
            }
            const args = await withUploadTimeout(
                uploadAsset(target, base64Body, settings)
            );
            if (!isCurrentInventoryAuthTarget(authTarget)) {
                return;
            }
            const key = scopeKey(target, 'custom');
            if (args?.json && typeof args.json.id === 'string') {
                const uploadedFile: MediaFileRecord = {
                    ...args.json,
                    id: args.json.id
                };
                setRowsByScope((current) => ({
                    ...current,
                    [key]: [
                        uploadedFile,
                        ...(current[key] || []).filter(
                            (item) => item.id !== uploadedFile.id
                        )
                    ]
                }));
            } else {
                await refreshScope(target, 'custom');
            }
            toast.success(t('message.upload.success'));
        } catch (error) {
            if (isCurrentInventoryAuthTarget(authTarget)) {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : t('message.upload.error')
                );
            }
        } finally {
            setUploadingTarget('');
            uploadTargetRef.current = null;
            setCropRequest(null);
        }
    }

    async function deleteFileAsset(fileId: unknown) {
        const normalizedFileId =
            typeof fileId === 'string'
                ? fileId.trim()
                : String(fileId ?? '').trim();
        if (!normalizedFileId) {
            return;
        }
        const result = await confirm({
            title: t('view.tools.modal.delete_value_item', {
                value: activeCategory
            }),
            description: normalizedFileId,
            confirmText: t('common.actions.delete'),
            cancelText: t('common.actions.cancel'),
            destructive: true
        });
        if (!result.ok) {
            return;
        }
        const authTarget = getInventoryAuthTarget();
        setMutatingKey(`file:${normalizedFileId}`);
        try {
            await mediaRepository.deleteFile(normalizedFileId);
            if (isCurrentInventoryAuthTarget(authTarget)) {
                setRowsByScope((current) => ({
                    ...current,
                    [activeScopeKey]: (current[activeScopeKey] || []).filter(
                        (file) => file.id !== normalizedFileId
                    )
                }));
                toast.success(t('view.tools.success.media_item_deleted'));
            }
        } catch (error) {
            if (isCurrentInventoryAuthTarget(authTarget)) {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : t('view.tools.toast.failed_to_delete_media_item')
                );
            }
        } finally {
            setMutatingKey((current) =>
                current === `file:${normalizedFileId}` ? '' : current
            );
        }
    }

    async function archiveInventoryItem(
        inventoryId: unknown,
        archived: boolean
    ) {
        const normalizedInventoryId =
            typeof inventoryId === 'string'
                ? inventoryId.trim()
                : String(inventoryId ?? '').trim();
        if (!normalizedInventoryId) {
            return;
        }
        const authTarget = getInventoryAuthTarget();
        setMutatingKey(`inventory:${normalizedInventoryId}`);
        try {
            await mediaRepository.updateInventoryItem(normalizedInventoryId, {
                isArchived: Boolean(archived)
            });
            if (isCurrentInventoryAuthTarget(authTarget)) {
                toast.success(
                    archived
                        ? t('dialog.inventory.archived_success')
                        : t('dialog.inventory.unarchived_success')
                );
                await refreshScope(activeCategory, activeSubTab);
            }
        } catch (error) {
            if (isCurrentInventoryAuthTarget(authTarget)) {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : t('dialog.inventory.failed_to_archive')
                );
            }
        } finally {
            setMutatingKey((current) =>
                current === `inventory:${normalizedInventoryId}` ? '' : current
            );
        }
    }

    async function consumeInventoryBundle(inventoryId: unknown) {
        const normalizedInventoryId =
            typeof inventoryId === 'string'
                ? inventoryId.trim()
                : String(inventoryId ?? '').trim();
        if (!normalizedInventoryId) {
            return;
        }
        const authTarget = getInventoryAuthTarget();
        setMutatingKey(`inventory:${normalizedInventoryId}`);
        try {
            await mediaRepository.consumeInventoryBundle(normalizedInventoryId);
            if (isCurrentInventoryAuthTarget(authTarget)) {
                toast.success(t('view.tools.label.inventory_bundle_consumed'));
                await refreshScope(activeCategory, activeSubTab);
            }
        } catch (error) {
            if (isCurrentInventoryAuthTarget(authTarget)) {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : t(
                              'view.tools.toast.failed_to_consume_inventory_bundle'
                          )
                );
            }
        } finally {
            setMutatingKey((current) =>
                current === `inventory:${normalizedInventoryId}` ? '' : current
            );
        }
    }

    async function setProfileDecorationEquipped(item: InventoryItemRecord) {
        const authTarget = getInventoryAuthTarget();
        if (inventoryAuthTargetKey(authTarget) !== currentAuthTargetKey) {
            return;
        }
        const mutation = resolveProfileDecorationMutation(
            item,
            authTarget.userId
        );
        if (!mutation) {
            return;
        }
        if (profileDecorationMutationPendingRef.current) {
            return;
        }

        profileDecorationMutationPendingRef.current = true;
        setProfileDecorationMutationPending(true);
        const mutationKey = `inventory:${mutation.inventoryId}`;
        const isUnequip = mutation.action === 'unequip';
        setMutatingKey(mutationKey);
        try {
            try {
                if (isUnequip) {
                    await mediaRepository.unequipProfileDecoration({
                        expectedUserId: authTarget.userId,
                        equipSlot: mutation.equipSlot
                    });
                } else {
                    await mediaRepository.equipProfileDecoration({
                        expectedUserId: authTarget.userId,
                        inventoryId: mutation.inventoryId,
                        equipSlot: mutation.equipSlot
                    });
                }
            } catch (error) {
                if (isCurrentInventoryAuthTarget(authTarget)) {
                    toast.error(
                        error instanceof Error
                            ? error.message
                            : t(
                                  'dialog.inventory.failed_to_update_profile_decoration'
                              )
                    );
                }
                return;
            }
            if (!isCurrentInventoryAuthTarget(authTarget)) {
                return;
            }

            toast.success(
                t(
                    isUnequip
                        ? 'dialog.inventory.unequipped_success'
                        : 'dialog.inventory.equipped_success'
                )
            );
            await Promise.allSettled([
                refreshScope('cosmetics', 'profile-decorations'),
                refreshCurrentUser({
                    expectedUserId: authTarget.userId,
                    expectedEndpoint: authTarget.endpoint,
                    expectedWebsocket: authTarget.websocket
                })
            ]);
        } finally {
            if (isCurrentInventoryAuthTarget(authTarget)) {
                profileDecorationMutationPendingRef.current = false;
                setProfileDecorationMutationPending(false);
                setMutatingKey((current) =>
                    current === mutationKey ? '' : current
                );
            }
        }
    }

    async function redeemReward() {
        const authTarget = getInventoryAuthTarget();
        const result = await prompt({
            title: t('prompt.redeem.header'),
            description: t('prompt.redeem.description'),
            confirmText: t('prompt.redeem.redeem'),
            cancelText: t('prompt.redeem.cancel')
        });
        if (!result.ok || !String(result.value || '').trim()) {
            return;
        }
        if (!isCurrentInventoryAuthTarget(authTarget)) {
            return;
        }
        setMutatingKey('inventory:redeem');
        try {
            await mediaRepository.redeemReward(result.value);
            if (isCurrentInventoryAuthTarget(authTarget)) {
                toast.success(t('prompt.redeem.success'));
                await refreshScope(activeCategory, activeSubTab);
            }
        } catch (error) {
            if (isCurrentInventoryAuthTarget(authTarget)) {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : t('view.tools.toast.failed_to_redeem_reward')
                );
            }
        } finally {
            setMutatingKey((current) =>
                current === 'inventory:redeem' ? '' : current
            );
        }
    }

    function closeCropRequest() {
        uploadTargetRef.current = null;
        setCropRequest(null);
    }

    return {
        activeCategory,
        activeSubTab,
        activeSubTabs,
        archiveInventoryItem,
        beginUpload,
        changeGridDensity,
        closeCropRequest,
        confirmCroppedUpload,
        consumeInventoryBundle,
        currentUserId,
        cropRequest,
        deleteFileAsset,
        emojiAnimFps,
        emojiAnimFrameCount,
        emojiAnimLoopPingPong,
        emojiAnimType,
        emojiAnimationStyle,
        gridDensity,
        gridDensityConfig,
        isVrcPlusSupporter,
        mutatingKey,
        openImagePreview,
        profileDecorationMutationPending,
        redeemReward,
        refreshScope,
        rowsByScope:
            scopeStateAuthTargetKey === currentAuthTargetKey
                ? rowsByScope
                : EMPTY_ROWS_BY_SCOPE,
        loadingByScope:
            scopeStateAuthTargetKey === currentAuthTargetKey
                ? loadingByScope
                : EMPTY_LOADING_BY_SCOPE,
        setActiveCategory,
        setActiveSubTabs,
        setEmojiAnimFps,
        setEmojiAnimFrameCount,
        setEmojiAnimLoopPingPong,
        setEmojiAnimType,
        setEmojiAnimationStyle,
        setProfileDecorationEquipped,
        uploadInputRef,
        uploadingTarget,
        uploadSelectedFile
    };
}
