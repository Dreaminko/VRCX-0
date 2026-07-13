import { toast } from 'sonner';

import { commands } from '@/platform/tauri/bindings';
import type { DeepLinkAction } from '@/platform/tauri/bindings';
import { tauriClient } from '@/platform/tauri/client';
import shareCollectionRepository from '@/repositories/shareCollectionRepository';
import { isCollectionShortcode } from '@/shared/constants/collectionShare';
import { isWorldId } from '@/shared/constants/vrchatIds';
import { useModalStore } from '@/state/modalStore';
import { useWorldCollectionImportStore } from '@/state/worldCollectionImportStore';

import { openWorldDialog } from './dialogService';
import { importWorldIdsToLocalGroup } from './favoriteImportService';
import i18n from './i18nService';

const DEEP_LINK_ARRIVED_EVENT = 'deepLinkArrived';
let sharedCollectionImportQueue = Promise.resolve();

type DeepLinkEventUnsubscribe = () => void;

export async function bindDeepLinkEvents(): Promise<DeepLinkEventUnsubscribe> {
    const unsubscribe = await tauriClient.events.subscribe(
        DEEP_LINK_ARRIVED_EVENT,
        () => {
            drainPendingDeepLinks().catch(logPendingDeepLinkDrainFailure);
        }
    );
    return unsubscribe;
}

export async function drainPendingDeepLinks(): Promise<void> {
    let actions: DeepLinkAction[];
    try {
        actions = await commands.appDrainPendingDeepLinks();
    } catch (error) {
        logPendingDeepLinkDrainFailure(error);
        return;
    }

    for (const action of actions) {
        handleDeepLinkAction(action);
    }
}

export function handleDeepLinkAction(action: DeepLinkAction): void {
    switch (action.type) {
        case 'openWorld':
            if (isWorldId(action.worldId)) {
                openWorldDialog({ worldId: action.worldId });
            } else {
                console.warn(
                    'Ignored deep link with invalid world id:',
                    action.worldId
                );
            }
            break;
        case 'importCollection':
            if (isCollectionShortcode(action.collectionId)) {
                sharedCollectionImportQueue = sharedCollectionImportQueue
                    .then(() => importSharedCollectionFlow(action.collectionId))
                    .catch((error) => {
                        console.warn(
                            'Failed to run shared collection import:',
                            error
                        );
                    });
            } else {
                console.warn(
                    'Ignored deep link with invalid collection id:',
                    action.collectionId
                );
            }
            break;
    }
}

function logPendingDeepLinkDrainFailure(error: unknown): void {
    console.warn('Failed to drain pending deep links:', error);
}

function errorMessage(error: unknown, fallback: string): string {
    return error instanceof Error && error.message ? error.message : fallback;
}

async function importSharedCollectionFlow(collectionId: string): Promise<void> {
    let preview;
    try {
        preview =
            await shareCollectionRepository.previewSharedCollection(
                collectionId
            );
    } catch (error) {
        toast.error(
            errorMessage(
                error,
                i18n.t('deep_link.import_collection.toast.preview_failed')
            )
        );
        return;
    }

    const worldCount = preview.worldIds.length;
    if (!worldCount) {
        toast.error(i18n.t('deep_link.import_collection.toast.empty'));
        return;
    }

    const prompt = await useModalStore.getState().prompt({
        title: i18n.t('deep_link.import_collection.prompt.title'),
        description: i18n.t('deep_link.import_collection.prompt.description', {
            count: worldCount
        }),
        inputValue: preview.title || collectionId,
        pattern: /\S/,
        confirmText: i18n.t('deep_link.import_collection.confirm.confirm'),
        cancelText: i18n.t('deep_link.import_collection.confirm.cancel')
    });
    if (!prompt.ok || typeof prompt.value !== 'string') {
        return;
    }
    const groupName = prompt.value.trim();
    if (!groupName) {
        return;
    }

    useWorldCollectionImportStore.getState().start(worldCount);
    try {
        const result = await importWorldIdsToLocalGroup({
            worldIds: preview.worldIds,
            groupName,
            onProgress: (progress) =>
                useWorldCollectionImportStore.getState().setProgress(progress)
        });
        if (!result.importedCount) {
            toast.error(
                i18n.t('deep_link.import_collection.toast.import_failed')
            );
            return;
        }
        toast.success(
            i18n.t('deep_link.import_collection.toast.import_success', {
                count: result.importedCount,
                title: groupName
            })
        );
        if (result.failedCount > 0) {
            toast.error(
                i18n.t(
                    'deep_link.import_collection.toast.import_partial_failed',
                    { count: result.failedCount }
                )
            );
        }
    } catch (error) {
        toast.error(
            errorMessage(
                error,
                i18n.t('deep_link.import_collection.toast.import_failed')
            )
        );
    } finally {
        useWorldCollectionImportStore.getState().finish();
    }
}
