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

export function useUserDialogProfileAppearance({
    profile
}: {
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
    const resourceKey = [
        userId,
        iconFrameId,
        profileEffectId,
        nameplateEffectId
    ].join('\u0000');
    const [resource, setResource] = useState<{
        key: string;
        value: UserDialogProfileAppearance;
    }>({
        key: '',
        value: EMPTY_APPEARANCE
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
        const templateIds = [
            ...new Set(Object.values(templateIdsBySlot).filter(Boolean))
        ];
        if (!userId || templateIds.length === 0) {
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
            const itemsByTemplateId = new Map<
                string,
                InventoryItemRecord | null
            >(
                results.map(({ inventoryTemplateId, item }) => [
                    inventoryTemplateId,
                    item
                ])
            );
            const value: UserDialogProfileAppearance = {};
            for (const slot of PROFILE_DECORATION_SLOTS) {
                const templateId = templateIdsBySlot[slot];
                const item = itemsByTemplateId.get(templateId);
                if (item) {
                    value[slot] = item;
                }
            }
            setResource({
                key: resourceKey,
                value
            });
        });

        return () => {
            active = false;
        };
    }, [resourceKey, templateIdsBySlot, userId]);

    return resource.key === resourceKey ? resource.value : EMPTY_APPEARANCE;
}
