import { BanIcon, PackageIcon } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
    EmptyState,
    LoadingState,
    PageBackButton,
    PageHeader,
    PageTitle,
    PageToolbar,
    PageToolbarRow
} from '@/components/layout/PageScaffold';
import { SelectableTile } from '@/components/tile/SelectableTile';
import {
    isEquippedProfileDecoration,
    resolveInventoryName,
    resolveProfileDecorationPreviewUrl,
    resolveProfileDecorationTypeLabelKey
} from '@/features/tools/inventoryHelpers';
import { cn } from '@/lib/utils';
import { ToggleGroup, ToggleGroupItem } from '@/ui/shadcn/toggle-group';

import {
    PROFILE_DECORATION_SLOTS,
    type ProfileDecorationSlot
} from '../userDialogProfileAppearance';
import type { UserDialogProfileRecord } from '../userDialogProfileTypes';
import {
    UNEQUIP_PENDING_KEY,
    type UserDialogProfileDecorationsController
} from '../useUserDialogProfileDecorations';
import { UserDialogProfileBackgroundPicker } from './UserDialogProfileBackgroundPicker';

type ProfileDecorationPanel = ProfileDecorationSlot | 'background';

function isProfileDecorationPanel(
    value: string
): value is ProfileDecorationPanel {
    return (
        value === 'background' ||
        PROFILE_DECORATION_SLOTS.some((slot) => slot === value)
    );
}

export function UserDialogProfileDecorationsPanel({
    profile,
    isVrcPlus,
    onBack,
    controller
}: {
    profile: UserDialogProfileRecord;
    isVrcPlus: boolean;
    onBack: () => void;
    controller: UserDialogProfileDecorationsController;
}) {
    const { t } = useTranslation();
    const [activeSlot, setActiveSlot] =
        useState<ProfileDecorationPanel>('iconFrame');
    const {
        itemsBySlot,
        pendingKey,
        isReady,
        equipItem,
        unequipSlot,
        updateBackground
    } = controller;

    const pending = Boolean(pendingKey);
    const isBackground = activeSlot === 'background';
    const decorationSlot = isBackground ? null : activeSlot;
    const items = decorationSlot ? itemsBySlot[decorationSlot] : [];
    const hasEquipped = items.some(isEquippedProfileDecoration);
    const isIconFrame = activeSlot === 'iconFrame';
    const isNameplate = activeSlot === 'nameplateEffect';
    const gridClassName = isNameplate
        ? 'grid-cols-1 sm:grid-cols-2'
        : 'grid-cols-3 sm:grid-cols-4';
    const tileAspectClassName = isNameplate ? 'aspect-[5/1]' : 'aspect-square';
    const tileContentClassName = isIconFrame
        ? undefined
        : isNameplate
          ? 'px-4 py-3'
          : 'p-3';
    const tileImageClassName = isIconFrame
        ? undefined
        : 'size-full object-cover';

    return (
        <div className="flex min-h-0 flex-1 flex-col gap-3">
            <PageToolbar>
                <PageToolbarRow className="items-center">
                    <PageBackButton
                        label={t('common.actions.back')}
                        onClick={onBack}
                    />
                    <PageHeader className="min-w-0 p-0">
                        <PageTitle>
                            {t('dialog.inventory.profile_decorations')}
                        </PageTitle>
                    </PageHeader>
                </PageToolbarRow>
            </PageToolbar>
            <ToggleGroup
                variant="outline"
                size="sm"
                spacing={1}
                value={[activeSlot]}
                onValueChange={(value) => {
                    const nextSlot = value[0];
                    if (nextSlot && isProfileDecorationPanel(nextSlot)) {
                        setActiveSlot(nextSlot);
                    }
                }}
                className="flex flex-wrap justify-start"
            >
                {[...PROFILE_DECORATION_SLOTS, 'background'].map((slot) => {
                    const label =
                        slot === 'background'
                            ? t('dialog.inventory.background')
                            : t(
                                  resolveProfileDecorationTypeLabelKey(slot) ??
                                      ''
                              );
                    return (
                        <ToggleGroupItem
                            key={slot}
                            value={slot}
                            aria-label={label}
                        >
                            {label}
                        </ToggleGroupItem>
                    );
                })}
            </ToggleGroup>
            <div className="min-h-0 flex-1 overflow-y-auto p-1">
                {isBackground ? (
                    <UserDialogProfileBackgroundPicker
                        profile={profile}
                        isVrcPlus={isVrcPlus || profile.hasVrcPlus === true}
                        pendingKey={pendingKey}
                        onUpdateBackground={updateBackground}
                    />
                ) : !isReady ? (
                    <LoadingState className="min-h-48" />
                ) : (
                    <div className="flex flex-col gap-3">
                        <div className={cn('grid gap-2', gridClassName)}>
                            <SelectableTile
                                label={t('dialog.gallery_select.none')}
                                icon={BanIcon}
                                isCurrent={!hasEquipped}
                                busy={pendingKey === UNEQUIP_PENDING_KEY}
                                inert={pending || !hasEquipped}
                                aspectClassName={tileAspectClassName}
                                surfaceClassName={tileContentClassName}
                                imageClassName={tileImageClassName}
                                onClick={() => {
                                    if (decorationSlot) {
                                        unequipSlot(decorationSlot);
                                    }
                                }}
                            />
                            {items.map((item) => {
                                const equipped =
                                    isEquippedProfileDecoration(item);
                                return (
                                    <SelectableTile
                                        key={item.id}
                                        label={resolveInventoryName(item)}
                                        imageUrl={resolveProfileDecorationPreviewUrl(
                                            item
                                        )}
                                        isCurrent={equipped}
                                        busy={pendingKey === item.id}
                                        inert={pending || equipped}
                                        aspectClassName={tileAspectClassName}
                                        surfaceClassName={tileContentClassName}
                                        imageClassName={tileImageClassName}
                                        onClick={() => equipItem(item)}
                                    />
                                );
                            })}
                        </div>
                        {!items.length ? (
                            <EmptyState
                                icon={PackageIcon}
                                className="min-h-32"
                                title={t('dialog.inventory.empty_title')}
                                description={t(
                                    'dialog.inventory.empty_description'
                                )}
                            />
                        ) : null}
                    </div>
                )}
            </div>
        </div>
    );
}
