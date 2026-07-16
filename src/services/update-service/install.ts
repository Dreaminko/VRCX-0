import type { TauriUpdateMetadata } from '@/platform/tauri/bindings';
import {
    discardPendingTauriUpdate,
    downloadAndInstallTauriUpdate,
    installPendingTauriUpdate
} from '@/platform/tauri/updater';

import {
    createTauriDownloadEventHandler,
    waitForInFlightDownload
} from './download';
import { buildTauriUpdaterRequest } from './tauriRequest';
import type { NormalizedRelease, UpdateOptions } from './types';

let updateInstallInFlight: Promise<TauriUpdateMetadata> | null = null;

export async function downloadAndInstallUpdate(
    release: NormalizedRelease,
    options: UpdateOptions = {}
): Promise<TauriUpdateMetadata> {
    if (updateInstallInFlight) {
        throw new Error('An update install is already in progress.');
    }
    const hostPlatform = options.hostPlatform || 'unknown';
    if (!release?.target) {
        throw new Error('Selected release has no Tauri updater target.');
    }

    updateInstallInFlight = (async () => {
        if (await waitForInFlightDownload(release.canonicalVersion, options)) {
            return installPendingTauriUpdate(release.canonicalVersion);
        }

        const request = await buildTauriUpdaterRequest(
            release,
            hostPlatform,
            options.hostArch || 'unknown',
            options.linuxPackageKind || 'unknown'
        );
        const onEvent = createTauriDownloadEventHandler(options);

        const update = await downloadAndInstallTauriUpdate(request, onEvent);
        if (!update) {
            throw new Error('No Tauri update is available.');
        }

        return update;
    })();

    try {
        return await updateInstallInFlight;
    } finally {
        updateInstallInFlight = null;
    }
}

export async function installPendingUpdate(
    version: string
): Promise<TauriUpdateMetadata> {
    return installPendingTauriUpdate(version);
}

export async function discardPendingUpdate(): Promise<void> {
    await discardPendingTauriUpdate();
}
