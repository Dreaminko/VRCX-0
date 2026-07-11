import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    downloadUpdate: vi.fn(),
    downloadAndInstallUpdate: vi.fn(),
    installPendingUpdate: vi.fn(),
    openExternalLink: vi.fn(),
    restartApplication: vi.fn(),
    toastError: vi.fn(),
    toastLoading: vi.fn(),
    toastSuccess: vi.fn()
}));

vi.mock('@/services/updateService', () => ({
    downloadUpdate: mocks.downloadUpdate,
    downloadAndInstallUpdate: mocks.downloadAndInstallUpdate,
    formatReleaseDisplayVersion: (value: unknown) => String(value || ''),
    installPendingUpdate: mocks.installPendingUpdate,
    isNoPendingUpdateError: (error: unknown) =>
        error instanceof Error && error.message.includes('no-pending-update'),
    isPendingUpdateVersionMismatchError: (error: unknown) =>
        error instanceof Error &&
        error.message.includes('pending-update-version-mismatch')
}));

vi.mock('@/services/entityMediaService', () => ({
    openExternalLink: mocks.openExternalLink
}));

vi.mock('@/services/shellIntegrationService', () => ({
    restartApplication: mocks.restartApplication
}));

vi.mock('@/services/i18nService', () => ({
    default: {
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key}:${JSON.stringify(values)}` : key
    }
}));

vi.mock('sonner', () => ({
    toast: {
        error: mocks.toastError,
        loading: mocks.toastLoading,
        success: mocks.toastSuccess
    }
}));

import { useRuntimeStore } from '@/state/runtimeStore';

import {
    installUpdateRelease,
    openOrInstallLatestAvailableUpdate
} from './updateInstallService';
import type { UpdateOptions } from './updateService';

function tauriRelease() {
    return {
        updaterType: 'tauri' as const,
        manifestUrl:
            'https://github.com/Map1en/VRCX-0/releases/latest/download/latest_windows.json',
        target: 'windows-x86_64-stable',
        channel: 'Stable' as const,
        htmlUrl: 'https://github.com/Map1en/VRCX-0/releases/tag/v2.7.0',
        canonicalVersion: '2.7.0',
        displayVersion: '2.7.0',
        tagName: 'v2.7.0',
        displayName: 'VRCX-0 2.7.0',
        prerelease: false,
        publishedAt: '2026-06-22T00:00:00Z',
        body: ''
    };
}

describe('openOrInstallLatestAvailableUpdate', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        useRuntimeStore.getState().resetRuntimeState();
        useRuntimeStore.getState().setHostCapabilities({
            platform: 'windows',
            arch: 'x86_64',
            linuxPackageKind: ''
        });
    });

    it('opens GitHub for a manual preview update release', async () => {
        useRuntimeStore.getState().setUpdateLoopState({
            latestUpdaterRelease: {
                updaterType: 'manual',
                htmlUrl: 'https://github.com/Map1en/VRCX-0/releases/tag/v2.7.0',
                canonicalVersion: '2.7.0',
                displayVersion: '2.7.0',
                tagName: 'v2.7.0',
                displayName: 'VRCX-0 2.7.0'
            }
        });

        await openOrInstallLatestAvailableUpdate();

        expect(mocks.openExternalLink).toHaveBeenCalledWith(
            'https://github.com/Map1en/VRCX-0/releases/tag/v2.7.0'
        );
        expect(mocks.downloadAndInstallUpdate).not.toHaveBeenCalled();
        expect(mocks.toastError).not.toHaveBeenCalled();
    });

    it('keeps installing when the latest update has Tauri updater metadata', async () => {
        useRuntimeStore.getState().setUpdateLoopState({
            latestUpdaterRelease: {
                updaterType: 'tauri',
                manifestUrl:
                    'https://github.com/Map1en/VRCX-0/releases/latest/download/latest_windows.json',
                target: 'windows-x86_64-stable',
                htmlUrl: 'https://github.com/Map1en/VRCX-0/releases/tag/v2.7.0',
                canonicalVersion: '2.7.0',
                displayVersion: '2.7.0',
                tagName: 'v2.7.0',
                displayName: 'VRCX-0 2.7.0'
            }
        });
        mocks.downloadAndInstallUpdate.mockResolvedValue({});

        await openOrInstallLatestAvailableUpdate();

        expect(mocks.downloadAndInstallUpdate).toHaveBeenCalled();
        expect(mocks.openExternalLink).not.toHaveBeenCalled();
    });

    it('installs a passed Tauri update release and restarts', async () => {
        mocks.downloadAndInstallUpdate.mockImplementation(
            async (_release: unknown, options: UpdateOptions) => {
                options.onDownloadProgress?.({
                    downloadedBytes: 50,
                    totalBytes: 100,
                    percent: 50
                });
                return {};
            }
        );

        const installed = await installUpdateRelease({
            updaterType: 'tauri',
            manifestUrl:
                'https://github.com/Map1en/VRCX-0/releases/latest/download/latest_windows.json',
            target: 'windows-x86_64-stable',
            channel: 'Stable',
            htmlUrl: 'https://github.com/Map1en/VRCX-0/releases/tag/v2.7.0',
            canonicalVersion: '2.7.0',
            displayVersion: '2.7.0',
            tagName: 'v2.7.0',
            displayName: 'VRCX-0 2.7.0',
            prerelease: false,
            publishedAt: '2026-06-22T00:00:00Z',
            body: ''
        });

        expect(installed).toBe(true);
        expect(mocks.downloadAndInstallUpdate).toHaveBeenCalled();
        expect(mocks.toastLoading).toHaveBeenCalledWith(
            expect.any(String),
            expect.objectContaining({ description: expect.anything() })
        );
        expect(mocks.toastSuccess).toHaveBeenCalled();
        expect(mocks.restartApplication).toHaveBeenCalled();
    });

    it('installs a matching downloaded update without downloading again', async () => {
        useRuntimeStore.getState().setUpdateLoopState({
            autoDownloadState: 'downloaded',
            downloadedVersion: '2.7.0',
            downloadProgress: 100
        });
        mocks.installPendingUpdate.mockResolvedValue({});

        const installed = await installUpdateRelease(tauriRelease());

        expect(installed).toBe(true);
        expect(mocks.installPendingUpdate).toHaveBeenCalledWith('2.7.0');
        expect(mocks.downloadAndInstallUpdate).not.toHaveBeenCalled();
        expect(mocks.restartApplication).toHaveBeenCalled();
    });

    it('falls back to direct download when the pending update is missing', async () => {
        useRuntimeStore.getState().setUpdateLoopState({
            autoDownloadState: 'downloaded',
            downloadedVersion: '2.7.0',
            downloadProgress: 100
        });
        mocks.installPendingUpdate.mockRejectedValue(
            new Error('no-pending-update')
        );
        mocks.downloadAndInstallUpdate.mockResolvedValue({});

        const installed = await installUpdateRelease(tauriRelease());

        expect(installed).toBe(true);
        expect(mocks.installPendingUpdate).toHaveBeenCalledWith('2.7.0');
        expect(mocks.downloadAndInstallUpdate).toHaveBeenCalled();
        expect(mocks.restartApplication).toHaveBeenCalled();
    });

    it('clears downloaded update state when pending install fails', async () => {
        useRuntimeStore.getState().setUpdateLoopState({
            autoDownloadState: 'downloaded',
            downloadedVersion: '2.7.0',
            downloadProgress: 100
        });
        mocks.installPendingUpdate.mockRejectedValue(
            new Error('access denied')
        );

        const installed = await installUpdateRelease(tauriRelease());

        expect(installed).toBe(false);
        expect(useRuntimeStore.getState().updateLoop.autoDownloadState).toBe(
            'idle'
        );
        expect(useRuntimeStore.getState().updateLoop.downloadedVersion).toBe(
            null
        );
        expect(mocks.restartApplication).not.toHaveBeenCalled();
    });

    it('rejects a passed manual update release without installing', async () => {
        const installed = await installUpdateRelease({
            updaterType: 'manual',
            channel: 'Stable',
            htmlUrl: 'https://github.com/Map1en/VRCX-0/releases/tag/v2.7.0',
            canonicalVersion: '2.7.0',
            displayVersion: '2.7.0',
            tagName: 'v2.7.0',
            displayName: 'VRCX-0 2.7.0',
            prerelease: false,
            publishedAt: '2026-06-22T00:00:00Z',
            body: ''
        });

        expect(installed).toBe(false);
        expect(mocks.downloadAndInstallUpdate).not.toHaveBeenCalled();
        expect(mocks.restartApplication).not.toHaveBeenCalled();
        expect(mocks.toastError).toHaveBeenCalledWith(
            'message.vrcx_updater.no_downloadable_releases_found',
            expect.objectContaining({
                position: 'bottom-right'
            })
        );
    });
});
