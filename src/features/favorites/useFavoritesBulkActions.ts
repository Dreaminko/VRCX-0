import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import type { FavoriteTransferItemResult } from '@/platform/tauri/bindings';
import favoriteTransferRepository from '@/repositories/favoriteTransferRepository';
import { useModalStore } from '@/state/modalStore';

import type {
    FavoriteGroup,
    FavoriteItem,
    FavoriteKind,
    FavoriteSource
} from './favoritesTypes';
import {
    buildFavoriteTransferFailureDescription,
    buildFavoriteTransferInput,
    buildFavoriteTransferSuccessfulKeys,
    buildFavoriteTransferTargets,
    groupFavoriteItemsBySourceGroup,
    resolveFavoriteSourceGroup
} from './favoriteTransfer';

export function useFavoritesBulkActions({
    currentEndpoint,
    handleRemoveLocalFavorite,
    handleRemoveRemoteFavorite,
    kind,
    localGroups,
    refreshFavorites,
    remoteGroups,
    selectedContentItems,
    selectedGroupKey,
    selectedSource,
    setSelectedKeys
}: {
    currentEndpoint: string;
    handleRemoveLocalFavorite(
        item: FavoriteItem,
        options?: { silent?: boolean }
    ): Promise<boolean>;
    handleRemoveRemoteFavorite(
        item: FavoriteItem,
        options?: { silent?: boolean }
    ): Promise<boolean>;
    kind: FavoriteKind;
    localGroups: FavoriteGroup[];
    refreshFavorites(options?: { silent?: boolean }): Promise<void>;
    remoteGroups: FavoriteGroup[];
    selectedContentItems: FavoriteItem[];
    selectedGroupKey: string;
    selectedSource: FavoriteSource;
    setSelectedKeys(value: string[] | ((current: string[]) => string[])): void;
}) {
    const { t } = useTranslation();
    const confirm = useModalStore((state) => state.confirm);
    const moveTargets = useMemo(
        () =>
            buildFavoriteTransferTargets({
                remoteGroups,
                localGroups,
                selectedSource,
                selectedGroupKey
            }),
        [localGroups, remoteGroups, selectedGroupKey, selectedSource]
    );

    async function bulkRemoveSelection() {
        if (!selectedContentItems.length) {
            return;
        }
        const result = await confirm({
            title: t('view.favorites.modal.delete_value_favorites', {
                value: selectedContentItems.length
            }),
            description: t('view.favorites.modal.this_action_cannot_be_undone'),
            destructive: true,
            confirmText: t('common.actions.delete'),
            cancelText: t('common.actions.cancel')
        });
        if (!result.ok) {
            return;
        }
        let removedCount = 0;
        let failedCount = 0;
        const removedKeys = new Set<string>();
        for (const item of selectedContentItems) {
            try {
                const removed =
                    item.source === 'local'
                        ? await handleRemoveLocalFavorite(item, {
                              silent: true
                          })
                        : await handleRemoveRemoteFavorite(item, {
                              silent: true
                          });
                if (removed) {
                    removedCount += 1;
                    removedKeys.add(item.key);
                } else {
                    failedCount += 1;
                }
            } catch {
                failedCount += 1;
            }
        }
        if (removedCount > 0) {
            setSelectedKeys((current) =>
                current.filter((key) => !removedKeys.has(key))
            );
        }
        if (failedCount === 0) {
            toast.success(
                t('view.favorite.success.selected_favorites_removed')
            );
            return;
        }
        toast.error(
            t('view.favorites.dynamic.removed_value_value_failed', {
                value: removedCount,
                value2: failedCount
            })
        );
    }

    async function bulkMoveSelection(targetGroup: FavoriteGroup) {
        if (!selectedContentItems.length) {
            return;
        }
        const batches = groupFavoriteItemsBySourceGroup(selectedContentItems);
        let succeeded = 0;
        let failed = 0;
        let sawCopy = false;
        let sawMove = false;
        const successfulKeys = new Set<string>();
        const failedResults: FavoriteTransferItemResult[] = [];
        let thrownErrorMessage = '';

        for (const batchItems of batches) {
            const sourceGroup = resolveFavoriteSourceGroup({
                item: batchItems[0],
                remoteGroups,
                localGroups
            });
            try {
                const result =
                    await favoriteTransferRepository.transferFavorites(
                        buildFavoriteTransferInput({
                            endpoint: currentEndpoint,
                            kind,
                            sourceGroup,
                            targetGroup,
                            selectedItems: batchItems
                        })
                    );
                succeeded += result.succeeded;
                failed += result.failed;
                for (const key of buildFavoriteTransferSuccessfulKeys(
                    result.items
                )) {
                    successfulKeys.add(key);
                }
                for (const item of result.items) {
                    if (item.status === 'failed') {
                        failedResults.push(item);
                    }
                }
                if (
                    sourceGroup.source === 'local' &&
                    targetGroup.source === 'remote'
                ) {
                    sawCopy = true;
                } else {
                    sawMove = true;
                }
            } catch (error) {
                failed += batchItems.length;
                if (!thrownErrorMessage && error instanceof Error) {
                    thrownErrorMessage = error.message;
                }
            }
        }

        if (succeeded > 0) {
            await refreshFavorites({ silent: true });
            setSelectedKeys((current) =>
                current.filter((key) => !successfulKeys.has(key))
            );
        }
        if (failed === 0) {
            const successMessage =
                sawCopy && !sawMove
                    ? t('view.favorite.success.selected_favorites_copied')
                    : t('view.favorite.success.selected_favorites_moved');
            toast.success(successMessage);
            return;
        }
        const fallbackMessage =
            thrownErrorMessage ||
            t('view.favorites.toast.failed_to_move_selected_favorites');
        const description = buildFavoriteTransferFailureDescription({
            results: failedResults,
            selectedItems: selectedContentItems,
            fallbackMessage
        });
        toast.error(
            t('view.favorites.dynamic.transferred_value_value_failed', {
                value: succeeded,
                value2: failed
            }),
            description ? { description } : undefined
        );
    }

    return {
        bulkMoveSelection,
        bulkRemoveSelection,
        moveTargets
    };
}
