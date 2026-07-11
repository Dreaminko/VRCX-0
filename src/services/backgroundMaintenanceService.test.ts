import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getConfigString: vi.fn(),
    getConfigBool: vi.fn(),
    setConfigString: vi.fn(),
    isHostCapabilityAvailable: vi.fn(),
    canInstallUpdatesOnPlatform: vi.fn(),
    checkInstallableUpdate: vi.fn(),
    discardPendingUpdate: vi.fn(),
    defaultBranchForVersion: vi.fn(),
    downloadUpdate: vi.fn(),
    fetchLatestBranchRelease: vi.fn(),
    formatReleaseDisplayVersion: vi.fn(),
    handlePreviewStableReleaseUpdateCheck: vi.fn(),
    hasUpdateForBranch: vi.fn(),
    runRuntimeTelemetryJob: vi.fn(),
    recordRuntimeJobTelemetry: vi.fn(),
    installUpdateRelease: vi.fn(),
    toastSuccess: vi.fn()
}));

vi.mock('@/repositories/configRepository', () => ({
    default: {
        getString: mocks.getConfigString,
        getBool: mocks.getConfigBool,
        setString: mocks.setConfigString
    }
}));

vi.mock('./hostCapabilityService', () => ({
    isHostCapabilityAvailable: mocks.isHostCapabilityAvailable
}));

vi.mock('./runtimeJobTelemetryService', () => ({
    recordRuntimeJobTelemetry: mocks.recordRuntimeJobTelemetry,
    runRuntimeTelemetryJob: mocks.runRuntimeTelemetryJob
}));

vi.mock('./updateService', () => ({
    canInstallUpdatesOnPlatform: mocks.canInstallUpdatesOnPlatform,
    checkInstallableUpdate: mocks.checkInstallableUpdate,
    discardPendingUpdate: mocks.discardPendingUpdate,
    defaultBranchForVersion: mocks.defaultBranchForVersion,
    downloadUpdate: mocks.downloadUpdate,
    fetchLatestBranchRelease: mocks.fetchLatestBranchRelease,
    formatReleaseDisplayVersion: mocks.formatReleaseDisplayVersion,
    handlePreviewStableReleaseUpdateCheck:
        mocks.handlePreviewStableReleaseUpdateCheck,
    hasUpdateForBranch: mocks.hasUpdateForBranch,
    sanitizeBranch: (branch: unknown) => String(branch || 'Stable')
}));

vi.mock('./updateInstallService', () => ({
    UPDATE_AVAILABLE_TOAST_ID: 'vrcx-update-available',
    installUpdateRelease: mocks.installUpdateRelease
}));

vi.mock('./i18nService', () => ({
    default: {
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key}:${JSON.stringify(values)}` : key
    }
}));

vi.mock('sonner', () => ({
    toast: {
        success: mocks.toastSuccess
    }
}));

import { useRuntimeStore } from '@/state/runtimeStore';

import {
    handleAutoBackgroundDownloadUpdatesPreferenceChange,
    runStartupMaintenance
} from './backgroundMaintenanceService';

function setAutoInstallUpdatesOnStartup(enabled: boolean) {
    mocks.getConfigBool.mockImplementation(
        async (key: string, defaultValue: boolean) =>
            key === 'autoInstallUpdatesOnStartup' ? enabled : defaultValue
    );
}

function useDefaultAutoInstallUpdatesOnStartup() {
    mocks.getConfigBool.mockImplementation(
        async (_key: string, defaultValue: boolean) => defaultValue
    );
}

describe('backgroundMaintenanceService update checks', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.stubGlobal('VERSION', '2.6.0');
        useRuntimeStore.getState().resetRuntimeState();
        useRuntimeStore.getState().setHostCapabilities({
            platform: 'windows',
            arch: 'x86_64',
            linuxPackageKind: ''
        });
        mocks.getConfigString.mockImplementation(
            async (key: string, defaultValue: string) => {
                if (key === 'autoUpdateVRCX') {
                    return 'Off';
                }
                return defaultValue;
            }
        );
        useDefaultAutoInstallUpdatesOnStartup();
        mocks.setConfigString.mockResolvedValue(undefined);
        mocks.isHostCapabilityAvailable.mockReturnValue(false);
        mocks.canInstallUpdatesOnPlatform.mockReturnValue(true);
        mocks.defaultBranchForVersion.mockReturnValue('Stable');
        mocks.formatReleaseDisplayVersion.mockImplementation((value: unknown) =>
            String(value || '')
        );
        mocks.checkInstallableUpdate.mockResolvedValue({
            canonicalVersion: '2.7.0',
            displayVersion: '2.7.0',
            htmlUrl: 'https://example.test/release',
            tagName: 'v2.7.0',
            displayName: 'VRCX-0 2.7.0',
            prerelease: false,
            publishedAt: '2026-06-18T00:00:00Z',
            body: '',
            updaterType: 'tauri',
            manifestUrl:
                'https://github.com/Map1en/VRCX-0/releases/latest/download/latest_windows.json',
            target: 'windows-x86_64-stable',
            currentVersion: '2.6.0',
            version: '2.7.0',
            date: null,
            rawJson: {}
        });
        mocks.fetchLatestBranchRelease.mockResolvedValue(null);
        mocks.hasUpdateForBranch.mockReturnValue(false);
        mocks.handlePreviewStableReleaseUpdateCheck.mockResolvedValue({
            handled: false,
            release: null
        });
        mocks.installUpdateRelease.mockResolvedValue(true);
        mocks.downloadUpdate.mockResolvedValue({});
        mocks.discardPendingUpdate.mockResolvedValue(undefined);
        mocks.runRuntimeTelemetryJob.mockImplementation(
            async (_metadata: unknown, task: () => Promise<unknown>) => task()
        );
    });

    it('installs a startup update by default without reading the old update mode', async () => {
        await runStartupMaintenance();

        expect(mocks.checkInstallableUpdate).toHaveBeenCalledWith('Stable', {
            hostArch: 'x86_64',
            linuxPackageKind: '',
            hostPlatform: 'windows'
        });
        expect(mocks.getConfigString).not.toHaveBeenCalledWith(
            'autoUpdateVRCX',
            expect.anything()
        );
        expect(mocks.setConfigString).not.toHaveBeenCalledWith(
            'autoUpdateVRCX',
            expect.anything()
        );
        expect(mocks.installUpdateRelease).toHaveBeenCalledWith(
            expect.objectContaining({
                updaterType: 'tauri',
                version: '2.7.0'
            })
        );
        expect(useRuntimeStore.getState().updateLoop.hasAvailableUpdate).toBe(
            false
        );
    });

    it('only reports a startup update when the new setting is disabled', async () => {
        setAutoInstallUpdatesOnStartup(false);

        await runStartupMaintenance();

        expect(mocks.installUpdateRelease).not.toHaveBeenCalled();
        expect(useRuntimeStore.getState().updateLoop.hasAvailableUpdate).toBe(
            true
        );
    });

    it('installs a startup update automatically when the new setting is enabled', async () => {
        setAutoInstallUpdatesOnStartup(true);

        await runStartupMaintenance();

        expect(mocks.installUpdateRelease).toHaveBeenCalled();
        expect(useRuntimeStore.getState().updateLoop.hasAvailableUpdate).toBe(
            false
        );
    });

    it('falls back to the available update notification when automatic install fails', async () => {
        setAutoInstallUpdatesOnStartup(true);
        mocks.installUpdateRelease.mockResolvedValue(false);

        await runStartupMaintenance();

        expect(mocks.installUpdateRelease).toHaveBeenCalled();
        expect(useRuntimeStore.getState().updateLoop.hasAvailableUpdate).toBe(
            true
        );
    });

    it('does not background-download by default when startup auto-install is disabled', async () => {
        mocks.getConfigBool.mockImplementation(
            async (key: string, defaultValue: boolean) => {
                if (key === 'autoInstallUpdatesOnStartup') {
                    return false;
                }
                return defaultValue;
            }
        );

        await runStartupMaintenance();

        expect(mocks.downloadUpdate).not.toHaveBeenCalled();
        expect(useRuntimeStore.getState().updateLoop.autoDownloadState).toBe(
            'idle'
        );
    });

    it('background-downloads an installable update when the new setting is enabled', async () => {
        mocks.getConfigBool.mockImplementation(
            async (key: string, defaultValue: boolean) => {
                if (key === 'autoInstallUpdatesOnStartup') {
                    return false;
                }
                if (key === 'autoBackgroundDownloadUpdates') {
                    return true;
                }
                return defaultValue;
            }
        );

        await handleAutoBackgroundDownloadUpdatesPreferenceChange(true);

        expect(mocks.downloadUpdate).toHaveBeenCalledWith(
            expect.objectContaining({
                canonicalVersion: '2.7.0',
                updaterType: 'tauri'
            }),
            expect.objectContaining({
                hostArch: 'x86_64',
                hostPlatform: 'windows'
            })
        );
        expect(useRuntimeStore.getState().updateLoop.autoDownloadState).toBe(
            'downloaded'
        );
        expect(useRuntimeStore.getState().updateLoop.downloadedVersion).toBe(
            '2.7.0'
        );
    });

    it('discards a stale downloaded update before downloading the latest version', async () => {
        mocks.getConfigBool.mockImplementation(
            async (key: string, defaultValue: boolean) => {
                if (key === 'autoInstallUpdatesOnStartup') {
                    return false;
                }
                if (key === 'autoBackgroundDownloadUpdates') {
                    return true;
                }
                return defaultValue;
            }
        );
        useRuntimeStore.getState().setUpdateLoopState({
            autoDownloadState: 'downloaded',
            downloadedVersion: '2.6.5',
            downloadProgress: 100
        });

        await handleAutoBackgroundDownloadUpdatesPreferenceChange(true);

        expect(mocks.discardPendingUpdate).toHaveBeenCalled();
        expect(mocks.downloadUpdate).toHaveBeenCalled();
        expect(useRuntimeStore.getState().updateLoop.downloadedVersion).toBe(
            '2.7.0'
        );
    });

    it('uses the preview stable release check without invoking the Tauri updater path', async () => {
        mocks.handlePreviewStableReleaseUpdateCheck.mockResolvedValue({
            handled: true,
            release: {
                canonicalVersion: '2.7.0',
                displayVersion: '2.7.0',
                htmlUrl: 'https://example.test/release',
                tagName: 'v2.7.0',
                displayName: 'VRCX-0 2.7.0',
                prerelease: false,
                publishedAt: '2026-06-21T07:00:00Z',
                body: '',
                updaterType: 'manual'
            }
        });

        await runStartupMaintenance();

        expect(
            mocks.handlePreviewStableReleaseUpdateCheck
        ).toHaveBeenCalledWith({
            hostArch: 'x86_64',
            linuxPackageKind: '',
            hostPlatform: 'windows'
        });
        expect(mocks.checkInstallableUpdate).not.toHaveBeenCalled();
        expect(useRuntimeStore.getState().updateLoop.hasAvailableUpdate).toBe(
            true
        );
        const latestUpdaterRelease =
            useRuntimeStore.getState().updateLoop.latestUpdaterRelease;
        expect(latestUpdaterRelease?.updaterType).toBe('manual');
        expect(mocks.installUpdateRelease).not.toHaveBeenCalled();
        expect(mocks.downloadUpdate).not.toHaveBeenCalled();
    });

    it('does not fall back to the Tauri updater path when a preview build has no stable release update', async () => {
        mocks.handlePreviewStableReleaseUpdateCheck.mockResolvedValue({
            handled: true,
            release: null
        });

        await runStartupMaintenance();

        expect(
            mocks.handlePreviewStableReleaseUpdateCheck
        ).toHaveBeenCalledWith({
            hostArch: 'x86_64',
            linuxPackageKind: '',
            hostPlatform: 'windows'
        });
        expect(mocks.checkInstallableUpdate).not.toHaveBeenCalled();
        expect(useRuntimeStore.getState().updateLoop.hasAvailableUpdate).toBe(
            false
        );
        expect(mocks.installUpdateRelease).not.toHaveBeenCalled();
    });
});
