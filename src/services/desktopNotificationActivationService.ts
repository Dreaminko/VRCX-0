import { commands } from '@/platform/tauri/bindings';
import { tauriClient } from '@/platform/tauri/client';
import { isUserId } from '@/shared/constants/vrchatIds';

import { openUserDialog } from './dialogService';

const DESKTOP_NOTIFICATION_ACTIVATED_EVENT = 'desktopNotificationActivated';

export async function bindDesktopNotificationActivationEvents(): Promise<
    () => void
> {
    return tauriClient.events.subscribe(
        DESKTOP_NOTIFICATION_ACTIVATED_EVENT,
        () => {
            takePendingDesktopNotificationActivation().catch(
                logDesktopNotificationActivationFailure
            );
        }
    );
}

export async function takePendingDesktopNotificationActivation(): Promise<void> {
    const activation =
        await commands.appTakePendingDesktopNotificationActivation();
    if (!activation) {
        return;
    }
    if (!isUserId(activation.userId)) {
        console.warn(
            'Ignored desktop notification activation with invalid user id:',
            activation.userId
        );
        return;
    }
    openUserDialog({ userId: activation.userId });
}

function logDesktopNotificationActivationFailure(error: unknown): void {
    console.warn(
        'Failed to take pending desktop notification activation:',
        error
    );
}
