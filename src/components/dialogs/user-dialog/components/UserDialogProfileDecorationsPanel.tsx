import { BanIcon, ImageIcon, PackageIcon } from 'lucide-react';
import { useState, type ComponentType } from 'react';
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
import { FadeInImage } from '@/components/media/FadeInImage';
import {
    isEquippedProfileDecoration,
    resolveInventoryName,
    resolveProfileDecorationPreviewUrl,
    resolveProfileDecorationTypeLabelKey
} from '@/features/tools/inventoryHelpers';
import { cn } from '@/lib/utils';
import { Button } from '@/ui/shadcn/button';
import { ToggleGroup, ToggleGroupItem } from '@/ui/shadcn/toggle-group';

import {
    PROFILE_DECORATION_SLOTS,
    type ProfileDecorationSlot
} from '../userDialogProfileAppearance';
import { useUserDialogProfileDecorations } from '../useUserDialogProfileDecorations';

function DecorationTile({
    label,
    imageUrl,
    icon: Icon,
    isCurrent,
    disabled,
    onClick
}: {
    label: string;
    imageUrl?: string;
    icon?: ComponentType<{ className?: string }>;
    isCurrent: boolean;
    disabled: boolean;
    onClick: () => void;
}) {
    return (
        <Button
            type="button"
            variant="ghost"
            disabled={disabled}
            aria-pressed={isCurrent}
            title={label}
            onClick={onClick}
            className={cn(
                'relative aspect-square h-auto w-full min-w-0 overflow-hidden rounded-lg border p-0 transition-transform active:scale-[0.97]',
                isCurrent && 'ring-primary ring-2'
            )}
        >
            <div className="bg-muted/30 text-muted-foreground flex size-full items-center justify-center overflow-hidden">
                {Icon ? (
                    <Icon className="size-6" />
                ) : imageUrl ? (
                    <FadeInImage
                        src={imageUrl}
                        alt={label}
                        loading="lazy"
                        className="size-full object-contain"
                        fallback={<ImageIcon className="size-6" />}
                    />
                ) : (
                    <ImageIcon className="size-6" />
                )}
            </div>
        </Button>
    );
}

export function UserDialogProfileDecorationsPanel({
    onBack
}: {
    onBack: () => void;
}) {
    const { t } = useTranslation();
    const [activeSlot, setActiveSlot] =
        useState<ProfileDecorationSlot>('iconFrame');
    const { itemsBySlot, pending, isReady, equipItem, unequipSlot } =
        useUserDialogProfileDecorations({ enabled: true });

    const items = itemsBySlot[activeSlot];
    const hasEquipped = items.some(isEquippedProfileDecoration);

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
                    if (nextSlot) {
                        setActiveSlot(nextSlot as ProfileDecorationSlot);
                    }
                }}
                className="flex flex-wrap justify-start"
            >
                {PROFILE_DECORATION_SLOTS.map((slot) => {
                    const label = t(
                        resolveProfileDecorationTypeLabelKey(slot) ?? ''
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
            <div className="min-h-0 flex-1 overflow-y-auto pr-1">
                {!isReady ? (
                    <LoadingState className="min-h-48" />
                ) : (
                    <div className="flex flex-col gap-3">
                        <div className="grid grid-cols-3 gap-2 sm:grid-cols-4">
                            <DecorationTile
                                label={t('dialog.gallery_select.none')}
                                icon={BanIcon}
                                isCurrent={!hasEquipped}
                                disabled={pending || !hasEquipped}
                                onClick={() => unequipSlot(activeSlot)}
                            />
                            {items.map((item) => {
                                const equipped =
                                    isEquippedProfileDecoration(item);
                                return (
                                    <DecorationTile
                                        key={item.id}
                                        label={resolveInventoryName(item)}
                                        imageUrl={resolveProfileDecorationPreviewUrl(
                                            item
                                        )}
                                        isCurrent={equipped}
                                        disabled={pending || equipped}
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
