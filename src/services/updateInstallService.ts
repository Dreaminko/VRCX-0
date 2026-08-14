import { toast } from 'sonner';

import { userFacingErrorMessage } from '@/lib/errorDisplay';
import { openExternalLink } from '@/services/entityMediaService';
import i18n from '@/services/i18nService';
import { restartApplication } from '@/services/shellIntegrationService';
import {
    confirmInstall,
    formatReleaseDisplayVersion,
    type AppUpdateDownloadProgressPayload,
    type AppUpdateDownloadStatusSnapshot,
    type AppUpdateInstalledPayload,
    type NormalizedRelease
} from '@/services/updateService';
import { links } from '@/shared/constants/link';
import { MINUTE_MS } from '@/shared/constants/time';
import { useRuntimeStore } from '@/state/runtimeStore';

import { UPDATE_READY_TOAST_DURATION_MS } from './backgroundMaintenanceTiming';

export const UPDATE_AVAILABLE_TOAST_ID = 'vrcx-update-available';

type DirectUpdateInstallOptions = {
    toastId?: string | number;
};

type UpdateUiState = {
    hasAvailableUpdate: boolean;
    latestUpdaterRelease: unknown;
    autoDownloadUiVisible: boolean;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function getString(value: unknown) {
    return typeof value === 'string' ? value : String(value || '');
}

export function shouldShowUpdateUi({
    hasAvailableUpdate,
    latestUpdaterRelease,
    autoDownloadUiVisible
}: UpdateUiState): boolean {
    if (!hasAvailableUpdate || !isRecord(latestUpdaterRelease)) {
        return false;
    }
    return (
        getString(latestUpdaterRelease.updaterType) !== 'tauri' ||
        autoDownloadUiVisible
    );
}

function readLatestUpdateRelease(): NormalizedRelease | null {
    const release = useRuntimeStore.getState().updateLoop.latestUpdaterRelease;
    if (!isRecord(release)) {
        return null;
    }

    return {
        manifestUrl: getString(release.manifestUrl).trim() || undefined,
        target: getString(release.target).trim() || undefined,
        canonicalVersion: getString(release.canonicalVersion),
        channel: 'Stable',
        displayVersion: getString(release.displayVersion),
        htmlUrl: getString(release.htmlUrl),
        tagName: getString(release.tagName),
        displayName: getString(release.displayName || release.title),
        prerelease: false,
        publishedAt: getString(release.publishedAt),
        body: '',
        updaterType:
            getString(release.updaterType) === 'tauri' ? 'tauri' : 'manual'
    };
}

function canInstallUpdateRelease(
    release: NormalizedRelease | null
): release is NormalizedRelease & {
    manifestUrl: string;
    target: string;
} {
    return Boolean(
        release &&
        release.updaterType === 'tauri' &&
        release.manifestUrl &&
        release.target
    );
}

const IDLE_DOWNLOAD_STATE = {
    autoDownloadState: 'idle',
    downloadedVersion: null,
    downloadProgress: 0,
    downloadedBytes: 0
};
const AUTO_DOWNLOAD_UI_DELAY_MS = 30 * MINUTE_MS;

let autoDownloadUiTimer: ReturnType<typeof setTimeout> | null = null;

function clearAutoDownloadUiTimer(): void {
    if (autoDownloadUiTimer === null) {
        return;
    }
    globalThis.clearTimeout(autoDownloadUiTimer);
    autoDownloadUiTimer = null;
}

function scheduleAutoDownloadUi(version: string, startedAt: string): void {
    clearAutoDownloadUiTimer();
    const startedAtMs = Date.parse(startedAt);
    const remainingDelay = Math.max(
        0,
        (Number.isFinite(startedAtMs) ? startedAtMs : Date.now()) +
            AUTO_DOWNLOAD_UI_DELAY_MS -
            Date.now()
    );
    autoDownloadUiTimer = globalThis.setTimeout(() => {
        autoDownloadUiTimer = null;
        const updateLoop = useRuntimeStore.getState().updateLoop;
        if (
            updateLoop.autoDownloadState !== 'downloading' ||
            updateLoop.downloadedVersion !== version ||
            updateLoop.autoDownloadStartedAt !== startedAt
        ) {
            return;
        }
        useRuntimeStore.getState().setUpdateLoopState({
            autoDownloadUiVisible: true
        });
    }, remainingDelay);
}

export function resetAutoDownloadUiDelay(): void {
    clearAutoDownloadUiTimer();
    useRuntimeStore.getState().setUpdateLoopState({
        autoDownloadStartedAt: null,
        autoDownloadUiVisible: false
    });
}

function resetUpdateLoopState() {
    clearAutoDownloadUiTimer();
    useRuntimeStore.getState().setUpdateLoopState({
        ...IDLE_DOWNLOAD_STATE,
        autoDownloadStartedAt: null,
        autoDownloadUiVisible: false,
        hasAvailableUpdate: false,
        latestUpdaterRelease: null
    });
}

function resetAutoDownloadInstallState() {
    useRuntimeStore.getState().setUpdateLoopState({ ...IDLE_DOWNLOAD_STATE });
}

let directInstallInFlight: Promise<boolean> | null = null;

export function installUpdateRelease(
    release: NormalizedRelease | null,
    { toastId = UPDATE_AVAILABLE_TOAST_ID }: DirectUpdateInstallOptions = {}
) {
    if (directInstallInFlight) {
        return directInstallInFlight;
    }

    if (!canInstallUpdateRelease(release)) {
        toast.error(
            i18n.t('message.vrcx_updater.no_downloadable_releases_found'),
            {
                id: toastId,
                position: 'bottom-right',
                closeButton: true
            }
        );
        return Promise.resolve(false);
    }

    toast.dismiss(toastId);

    directInstallInFlight = (async () => {
        try {
            await confirmInstall(release.canonicalVersion);
            return true;
        } catch (error) {
            resetAutoDownloadInstallState();
            toast.error(
                userFacingErrorMessage(
                    error,
                    i18n.t('message.vrcx_updater.failed_install')
                ),
                {
                    id: toastId,
                    duration: Infinity,
                    position: 'bottom-right',
                    closeButton: true
                }
            );
            return false;
        } finally {
            directInstallInFlight = null;
        }
    })();

    return directInstallInFlight;
}

export function installLatestAvailableUpdate(
    options: DirectUpdateInstallOptions = {}
) {
    return installUpdateRelease(readLatestUpdateRelease(), options);
}

export async function openOrInstallLatestAvailableUpdate(
    options: DirectUpdateInstallOptions = {}
) {
    const release = readLatestUpdateRelease();
    if (canInstallUpdateRelease(release)) {
        return installUpdateRelease(release, options);
    }

    await openExternalLink(release?.htmlUrl || links.releases);
    return false;
}

export function handleAppUpdateDownloadProgressEvent(
    payload: AppUpdateDownloadProgressPayload
) {
    applyAppUpdateDownloadState(payload);

    if (!directInstallInFlight || payload.phase !== 'downloaded') {
        return;
    }

    toast.loading(i18n.t('message.vrcx_updater.installing_update'), {
        id: UPDATE_AVAILABLE_TOAST_ID,
        duration: Infinity,
        position: 'bottom-right',
        dismissible: false
    });
}

export function handleAppUpdateDownloadStatusSnapshot(
    snapshot: AppUpdateDownloadStatusSnapshot
): void {
    applyAppUpdateDownloadState(snapshot);
}

function applyAppUpdateDownloadState(
    payload: Pick<
        AppUpdateDownloadStatusSnapshot,
        'phase' | 'version' | 'startedAt' | 'downloadedBytes' | 'percent'
    >
): void {
    const updateLoop = useRuntimeStore.getState().updateLoop;
    const startsNewDownload =
        payload.phase === 'downloading' &&
        (updateLoop.autoDownloadState !== 'downloading' ||
            updateLoop.downloadedVersion !== payload.version ||
            updateLoop.autoDownloadStartedAt !== payload.startedAt);
    let autoDownloadStartedAt: string | null = null;
    if (payload.phase === 'downloading') {
        autoDownloadStartedAt = startsNewDownload
            ? payload.startedAt || new Date().toISOString()
            : updateLoop.autoDownloadStartedAt;
    }
    const autoDownloadUiVisible =
        payload.phase === 'downloaded' ||
        payload.phase === 'installing' ||
        payload.phase === 'error'
            ? true
            : updateLoop.autoDownloadUiVisible;

    useRuntimeStore.getState().setUpdateLoopState({
        autoDownloadState: payload.phase,
        downloadedVersion: payload.version,
        downloadProgress: payload.percent,
        downloadedBytes: payload.downloadedBytes,
        autoDownloadStartedAt,
        autoDownloadUiVisible
    });

    if (
        payload.phase === 'downloading' &&
        autoDownloadStartedAt !== null &&
        !autoDownloadUiVisible
    ) {
        if (startsNewDownload || autoDownloadUiTimer === null) {
            if (payload.version) {
                scheduleAutoDownloadUi(payload.version, autoDownloadStartedAt);
            }
        }
    } else {
        clearAutoDownloadUiTimer();
    }
}

export function handleAppUpdateInstalledEvent(
    payload: AppUpdateInstalledPayload
) {
    resetUpdateLoopState();
    const displayVersion =
        formatReleaseDisplayVersion(payload.version) || payload.version;
    toast.success(
        i18n.t('dialog.vrcx_updater.ready_for_update', {
            value: displayVersion
        }),
        {
            id: UPDATE_AVAILABLE_TOAST_ID,
            description: undefined,
            duration: UPDATE_READY_TOAST_DURATION_MS,
            position: 'bottom-right'
        }
    );
    void restartApplication();
}
