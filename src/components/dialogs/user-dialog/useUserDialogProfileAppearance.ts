import { useEffect, useMemo, useState } from 'react';

import vrchatMediaRepository, {
    type InventoryItemRecord
} from '@/repositories/vrchatMediaRepository';

import {
    PROFILE_DECORATION_SLOTS,
    type UserDialogProfileAppearance
} from './userDialogProfileAppearance';
import type { UserDialogProfileRecord } from './userDialogProfileTypes';

const EMPTY_APPEARANCE: UserDialogProfileAppearance = Object.freeze({});
const EMPTY_TEMPLATE_ITEMS: ReadonlyMap<string, InventoryItemRecord | null> =
    new Map();

export function useUserDialogProfileAppearance({
    enabled = true,
    profile
}: {
    enabled?: boolean;
    profile: UserDialogProfileRecord | null | undefined;
}): UserDialogProfileAppearance {
    const userId = profile?.id?.trim() ?? '';
    const iconFrameId =
        typeof profile?.iconFrame === 'string' ? profile.iconFrame.trim() : '';
    const profileEffectId =
        typeof profile?.profileEffect === 'string'
            ? profile.profileEffect.trim()
            : '';
    const nameplateEffectId =
        typeof profile?.nameplateEffect === 'string'
            ? profile.nameplateEffect.trim()
            : '';
    const [resource, setResource] = useState<{
        itemsByTemplateId: ReadonlyMap<string, InventoryItemRecord | null>;
        userId: string;
    }>({
        itemsByTemplateId: EMPTY_TEMPLATE_ITEMS,
        userId: ''
    });

    const templateIdsBySlot = useMemo(
        () => ({
            iconFrame: iconFrameId,
            profileEffect: profileEffectId,
            nameplateEffect: nameplateEffectId
        }),
        [iconFrameId, nameplateEffectId, profileEffectId]
    );

    useEffect(() => {
        const itemsByTemplateId =
            resource.userId === userId
                ? resource.itemsByTemplateId
                : EMPTY_TEMPLATE_ITEMS;
        const templateIds = [
            ...new Set(Object.values(templateIdsBySlot).filter(Boolean))
        ].filter((templateId) => !itemsByTemplateId.has(templateId));
        if (!enabled || !userId || templateIds.length === 0) {
            return;
        }

        let active = true;
        Promise.all(
            templateIds.map(async (inventoryTemplateId) => {
                try {
                    const response =
                        await vrchatMediaRepository.getInventoryTemplate(
                            inventoryTemplateId
                        );
                    return {
                        inventoryTemplateId,
                        item: response.json
                    };
                } catch {
                    return {
                        inventoryTemplateId,
                        item: null
                    };
                }
            })
        ).then((results) => {
            if (!active) {
                return;
            }
            setResource((currentResource) => {
                const nextItemsByTemplateId = new Map(
                    currentResource.userId === userId
                        ? currentResource.itemsByTemplateId
                        : EMPTY_TEMPLATE_ITEMS
                );
                let changed = currentResource.userId !== userId;
                for (const { inventoryTemplateId, item } of results) {
                    if (!nextItemsByTemplateId.has(inventoryTemplateId)) {
                        nextItemsByTemplateId.set(inventoryTemplateId, item);
                        changed = true;
                    }
                }
                if (!changed) {
                    return currentResource;
                }
                return {
                    itemsByTemplateId: nextItemsByTemplateId,
                    userId
                };
            });
        });

        return () => {
            active = false;
        };
    }, [enabled, resource, templateIdsBySlot, userId]);

    return useMemo(() => {
        if (!enabled || resource.userId !== userId) {
            return EMPTY_APPEARANCE;
        }
        const value: UserDialogProfileAppearance = {};
        for (const slot of PROFILE_DECORATION_SLOTS) {
            const item = resource.itemsByTemplateId.get(
                templateIdsBySlot[slot]
            );
            if (item) {
                value[slot] = item;
            }
        }
        return value;
    }, [enabled, resource, templateIdsBySlot, userId]);
}
