import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import {
    UPDATE_AVAILABLE_TOAST_ID,
    openOrInstallLatestAvailableUpdate
} from '@/services/updateInstallService';
import { useRuntimeStore } from '@/state/runtimeStore';

function getReleaseProperty(release: unknown, key: string): unknown {
    return release && typeof release === 'object'
        ? Reflect.get(release, key)
        : undefined;
}

function getLatestUpdaterDisplayVersion(release: unknown) {
    return (
        String(
            getReleaseProperty(release, 'latestVersion') ||
                getReleaseProperty(release, 'displayVersion') ||
                getReleaseProperty(release, 'canonicalVersion') ||
                getReleaseProperty(release, 'tagName') ||
                ''
        ).trim() || '-'
    );
}

function formatUpdateVersion(version: string) {
    if (!version || version === '-') {
        return version;
    }
    return version.replace(/^v/i, '');
}

function getReleaseCanonicalVersion(release: unknown) {
    return String(getReleaseProperty(release, 'canonicalVersion') || '');
}

function isDownloadedUpdateReady({
    latestUpdaterRelease,
    autoDownloadState,
    downloadedVersion
}: {
    latestUpdaterRelease: unknown;
    autoDownloadState: string;
    downloadedVersion: string | null;
}) {
    const latestVersion = getReleaseCanonicalVersion(latestUpdaterRelease);
    return (
        autoDownloadState === 'downloaded' &&
        Boolean(latestVersion) &&
        downloadedVersion === latestVersion
    );
}

export function showUpdateAvailableToast({
    latestUpdaterRelease,
    t,
    onUpdate
}: {
    latestUpdaterRelease: unknown;
    t: (key: string, values?: Record<string, unknown>) => string;
    onUpdate: () => void;
}) {
    toast.info(
        t('service.background_maintenance.label.vrcx_update_available'),
        {
            id: UPDATE_AVAILABLE_TOAST_ID,
            icon: null,
            description: formatUpdateVersion(
                getLatestUpdaterDisplayVersion(latestUpdaterRelease)
            ),
            duration: Infinity,
            position: 'bottom-right',
            closeButton: true,
            dismissible: true,
            action: {
                label: t('nav_menu.update'),
                onClick: onUpdate
            }
        }
    );
}

export function showUpdateReadyToast({
    latestUpdaterRelease,
    t,
    onUpdate
}: {
    latestUpdaterRelease: unknown;
    t: (key: string, values?: Record<string, unknown>) => string;
    onUpdate: () => void;
}) {
    const version = formatUpdateVersion(
        getLatestUpdaterDisplayVersion(latestUpdaterRelease)
    );
    toast.success(
        t('dialog.vrcx_updater.ready_for_update', {
            value: version
        }),
        {
            id: UPDATE_AVAILABLE_TOAST_ID,
            description: undefined,
            duration: Infinity,
            position: 'bottom-right',
            closeButton: true,
            dismissible: true,
            action: {
                label: t('nav_menu.update_downloaded'),
                onClick: onUpdate
            }
        }
    );
}

export function UpdateAvailableToastHost(): null {
    const { t } = useTranslation();
    const hasAvailableUpdate = useRuntimeStore((state) =>
        Boolean(state.updateLoop.hasAvailableUpdate)
    );
    const latestUpdaterRelease = useRuntimeStore(
        (state) => state.updateLoop.latestUpdaterRelease
    );
    const autoDownloadState = useRuntimeStore(
        (state) => state.updateLoop.autoDownloadState
    );
    const downloadedVersion = useRuntimeStore(
        (state) => state.updateLoop.downloadedVersion
    );

    useEffect(() => {
        if (!hasAvailableUpdate || !latestUpdaterRelease) {
            toast.dismiss(UPDATE_AVAILABLE_TOAST_ID);
            return undefined;
        }

        const openLatestUpdate = () => {
            void openOrInstallLatestAvailableUpdate({
                toastId: UPDATE_AVAILABLE_TOAST_ID
            });
        };
        const showToast = isDownloadedUpdateReady({
            latestUpdaterRelease,
            autoDownloadState,
            downloadedVersion
        })
            ? showUpdateReadyToast
            : showUpdateAvailableToast;
        showToast({
            latestUpdaterRelease,
            t,
            onUpdate: openLatestUpdate
        });

        return undefined;
    }, [
        autoDownloadState,
        downloadedVersion,
        hasAvailableUpdate,
        latestUpdaterRelease,
        t
    ]);

    return null;
}
