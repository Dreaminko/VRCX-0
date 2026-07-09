import { commands } from '@/platform/tauri/bindings';
import type { DeepLinkAction } from '@/platform/tauri/bindings';
import { tauriClient } from '@/platform/tauri/client';
import { isCollectionShortcode } from '@/shared/constants/collectionShare';
import { isWorldId } from '@/shared/constants/vrchatIds';
import { useModalStore } from '@/state/modalStore';

import { openWorldDialog } from './dialogService';
import i18n from './i18nService';

const DEEP_LINK_ARRIVED_EVENT = 'deepLinkArrived';

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
                openCollectionImportPlaceholder(action.collectionId);
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

function openCollectionImportPlaceholder(collectionId: string): void {
    void useModalStore.getState().openAlert({
        title: i18n.t('deep_link.import_collection.title'),
        description: i18n.t('deep_link.import_collection.description', {
            collectionId
        }),
        confirmText: i18n.t('common.actions.close')
    });
}
