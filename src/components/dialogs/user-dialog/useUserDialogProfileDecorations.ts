import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { resolveProfileDecorationMutation } from '@/features/tools/inventoryHelpers';
import mediaRepository, {
    type InventoryItemRecord
} from '@/repositories/mediaRepository';
import { refreshCurrentUser } from '@/services/backgroundMaintenanceSessionService';
import { useRuntimeStore } from '@/state/runtimeStore';

import {
    PROFILE_DECORATION_SLOTS,
    type ProfileDecorationSlot
} from './userDialogProfileAppearance';

const PROFILE_DECORATION_TYPES_PARAM = PROFILE_DECORATION_SLOTS.join(',');

type ItemsBySlot = Record<ProfileDecorationSlot, InventoryItemRecord[]>;

type ProfileDecorationsAuthTarget = {
    endpoint: string;
    userId: string;
    websocket: string;
};

const EMPTY_ITEMS_BY_SLOT: ItemsBySlot = {
    iconFrame: [],
    profileEffect: [],
    nameplateEffect: []
};

function emptyItemsBySlot(): ItemsBySlot {
    return {
        iconFrame: [],
        profileEffect: [],
        nameplateEffect: []
    };
}

function authTargetKey(target: ProfileDecorationsAuthTarget) {
    return [target.endpoint, target.userId, target.websocket].join(' ');
}

function isProfileDecorationSlot(
    value: unknown
): value is ProfileDecorationSlot {
    return PROFILE_DECORATION_SLOTS.some((slot) => slot === value);
}

export function useUserDialogProfileDecorations({
    enabled
}: {
    enabled: boolean;
}) {
    const { t } = useTranslation();
    const tRef = useRef(t);
    tRef.current = t;
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const currentEndpoint = useRuntimeStore(
        (state) => state.auth.currentUserEndpoint
    );
    const currentUserWebsocket = useRuntimeStore(
        (state) => state.auth.currentUserWebsocket
    );

    const authTargetRef = useRef<ProfileDecorationsAuthTarget>({
        endpoint: '',
        userId: '',
        websocket: ''
    });
    authTargetRef.current = {
        endpoint: currentEndpoint || '',
        userId: currentUserId || '',
        websocket: currentUserWebsocket || ''
    };
    const currentKey = authTargetKey(authTargetRef.current);

    const [itemsBySlot, setItemsBySlot] =
        useState<ItemsBySlot>(emptyItemsBySlot);
    const [loadedKey, setLoadedKey] = useState('');
    const [loading, setLoading] = useState(false);
    const [pending, setPending] = useState(false);
    const pendingRef = useRef(false);

    const refresh = useCallback(async () => {
        const target = authTargetRef.current;
        const targetKey = authTargetKey(target);
        if (!target.userId) {
            return;
        }
        setLoading(true);
        try {
            const { items: rows, truncated } =
                await mediaRepository.collectInventoryItems({
                    order: 'newest',
                    types: PROFILE_DECORATION_TYPES_PARAM,
                    notFlags: 'ugc',
                    archived: false
                });
            if (truncated) {
                console.warn(
                    'Profile decoration listing truncated at the page limit.'
                );
            }
            if (authTargetKey(authTargetRef.current) !== targetKey) {
                return;
            }
            const next = emptyItemsBySlot();
            for (const row of rows) {
                if (isProfileDecorationSlot(row.itemType)) {
                    next[row.itemType].push(row);
                }
            }
            setItemsBySlot(next);
            setLoadedKey(targetKey);
        } catch (error) {
            if (authTargetKey(authTargetRef.current) === targetKey) {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : tRef.current('dialog.inventory.failed_to_load')
                );
            }
        } finally {
            if (authTargetKey(authTargetRef.current) === targetKey) {
                setLoading(false);
            }
        }
    }, []);

    useEffect(() => {
        if (!enabled || !currentKey) {
            return;
        }
        refresh();
    }, [currentKey, enabled, refresh]);

    async function runMutation({
        action,
        equipSlot,
        inventoryId
    }: {
        action: 'equip' | 'unequip';
        equipSlot: ProfileDecorationSlot;
        inventoryId: string;
    }) {
        const target = authTargetRef.current;
        if (!target.userId || pendingRef.current) {
            return;
        }
        pendingRef.current = true;
        setPending(true);
        const isUnequip = action === 'unequip';
        try {
            if (isUnequip) {
                await mediaRepository.unequipProfileDecoration({
                    expectedUserId: target.userId,
                    equipSlot
                });
            } else {
                await mediaRepository.equipProfileDecoration({
                    expectedUserId: target.userId,
                    inventoryId,
                    equipSlot
                });
            }
            toast.success(
                t(
                    isUnequip
                        ? 'dialog.inventory.unequipped_success'
                        : 'dialog.inventory.equipped_success'
                )
            );
            await Promise.allSettled([
                refresh(),
                refreshCurrentUser({
                    expectedUserId: target.userId,
                    expectedEndpoint: target.endpoint,
                    expectedWebsocket: target.websocket
                })
            ]);
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t('dialog.inventory.failed_to_update_profile_decoration')
            );
        } finally {
            pendingRef.current = false;
            setPending(false);
        }
    }

    function equipItem(item: InventoryItemRecord) {
        const mutation = resolveProfileDecorationMutation(
            item,
            authTargetRef.current.userId
        );
        if (!mutation || mutation.action !== 'equip') {
            return;
        }
        runMutation(mutation);
    }

    function unequipSlot(slot: ProfileDecorationSlot) {
        runMutation({ action: 'unequip', equipSlot: slot, inventoryId: '' });
    }

    const isReady = loadedKey === currentKey;

    return {
        itemsBySlot: enabled && isReady ? itemsBySlot : EMPTY_ITEMS_BY_SLOT,
        loading,
        pending,
        isReady,
        equipItem,
        unequipSlot
    };
}
