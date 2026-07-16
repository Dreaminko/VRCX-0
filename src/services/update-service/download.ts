import type {
    TauriDownloadEvent,
    TauriUpdateMetadata
} from '@/platform/tauri/bindings';
import { downloadTauriUpdate } from '@/platform/tauri/updater';

import { buildTauriUpdaterRequest } from './tauriRequest';
import type {
    NormalizedRelease,
    UpdateDownloadProgress,
    UpdateDownloadProgressSubscriber,
    UpdateOptions
} from './types';

type UpdateDownloadInFlight = {
    version: string;
    promise: Promise<TauriUpdateMetadata>;
    progressSubscribers: Set<UpdateDownloadProgressSubscriber>;
    lastProgress: UpdateDownloadProgress | null;
};

let updateDownloadInFlight: UpdateDownloadInFlight | null = null;

function handleTauriDownloadEvent(
    event: TauriDownloadEvent,
    emitProgress: UpdateDownloadProgressSubscriber
): { downloaded: number; contentLength: number } | null {
    if (event.event === 'Started') {
        const contentLength = Number(event.data?.contentLength) || 0;
        emitProgress({
            downloadedBytes: 0,
            totalBytes: contentLength,
            percent: 0
        });
        return {
            downloaded: 0,
            contentLength
        };
    }
    if (event.event === 'Finished') {
        emitProgress({
            downloadedBytes: 0,
            totalBytes: 0,
            percent: 100
        });
    }
    return null;
}

function emitUpdateDownloadProgress(
    options: UpdateOptions,
    progress: UpdateDownloadProgress
) {
    options.onProgress?.(progress.percent);
    options.onDownloadProgress?.(progress);
}

export function createTauriDownloadEventHandler(
    options: UpdateOptions = {},
    emitProgress: UpdateDownloadProgressSubscriber = (progress) =>
        emitUpdateDownloadProgress(options, progress)
): (event: TauriDownloadEvent) => void {
    let downloaded = 0;
    let contentLength = 0;
    return (event: TauriDownloadEvent) => {
        const state = handleTauriDownloadEvent(event, emitProgress);
        if (state) {
            downloaded = state.downloaded;
            contentLength = state.contentLength;
            return;
        }
        if (event.event !== 'Progress') {
            return;
        }
        downloaded += Number(event.data?.chunkLength) || 0;
        if (contentLength > 0) {
            const percent = Math.min(
                100,
                Math.round((downloaded / contentLength) * 100)
            );
            emitProgress({
                downloadedBytes: downloaded,
                totalBytes: contentLength,
                percent
            });
            return;
        }
        emitProgress({
            downloadedBytes: downloaded,
            totalBytes: 0,
            percent: 0
        });
    };
}

function publishInFlightDownloadProgress(
    download: UpdateDownloadInFlight,
    progress: UpdateDownloadProgress
) {
    download.lastProgress = progress;
    for (const subscriber of download.progressSubscribers) {
        subscriber(progress);
    }
}

function subscribeToInFlightDownload(
    download: UpdateDownloadInFlight,
    options: UpdateOptions
) {
    if (!options.onProgress && !options.onDownloadProgress) {
        return () => {};
    }
    const subscriber = (progress: UpdateDownloadProgress) => {
        emitUpdateDownloadProgress(options, progress);
    };
    download.progressSubscribers.add(subscriber);
    if (download.lastProgress) {
        subscriber(download.lastProgress);
    }
    return () => {
        download.progressSubscribers.delete(subscriber);
    };
}

export async function waitForInFlightDownload(
    version: string,
    options: UpdateOptions
): Promise<boolean> {
    const pendingDownload = updateDownloadInFlight;
    if (pendingDownload?.version !== version) {
        return false;
    }
    const unsubscribe = subscribeToInFlightDownload(pendingDownload, options);
    try {
        await pendingDownload.promise;
        return true;
    } finally {
        unsubscribe();
    }
}

export async function downloadUpdate(
    release: NormalizedRelease,
    options: UpdateOptions = {}
): Promise<TauriUpdateMetadata> {
    const version = release.canonicalVersion;
    if (!version) {
        throw new Error('Selected release has no canonical update version.');
    }
    if (updateDownloadInFlight) {
        if (updateDownloadInFlight.version === version) {
            const unsubscribe = subscribeToInFlightDownload(
                updateDownloadInFlight,
                options
            );
            try {
                return await updateDownloadInFlight.promise;
            } finally {
                unsubscribe();
            }
        }
        throw new Error('An update download is already in progress.');
    }
    const hostPlatform = options.hostPlatform || 'unknown';
    if (!release.target) {
        throw new Error('Selected release has no Tauri updater target.');
    }

    let inFlight: UpdateDownloadInFlight;
    const promise = (async () => {
        const request = await buildTauriUpdaterRequest(
            release,
            hostPlatform,
            options.hostArch || 'unknown',
            options.linuxPackageKind || 'unknown'
        );
        const update = await downloadTauriUpdate(
            version,
            request,
            createTauriDownloadEventHandler({}, (progress) =>
                publishInFlightDownloadProgress(inFlight, progress)
            )
        );
        if (!update) {
            throw new Error('No Tauri update is available.');
        }
        return update;
    })();

    inFlight = {
        version,
        promise,
        progressSubscribers: new Set(),
        lastProgress: null
    };
    updateDownloadInFlight = inFlight;
    const unsubscribe = subscribeToInFlightDownload(inFlight, options);
    try {
        return await promise;
    } finally {
        unsubscribe();
        if (updateDownloadInFlight?.promise === promise) {
            updateDownloadInFlight = null;
        }
    }
}
