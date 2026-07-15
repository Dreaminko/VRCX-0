// @vitest-environment jsdom

import { act, cleanup, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type {
    ProfileBackupActionOutcome,
    ProfileBackupSettings
} from '@/services/profileBackupService';

const mocks = vi.hoisted(() => ({
    getSettings: vi.fn(),
    runManual: vi.fn(),
    setSettings: vi.fn(),
    selectFolder: vi.fn(),
    toastError: vi.fn(),
    translate: (key: string) => key
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: mocks.translate })
}));

vi.mock('sonner', () => ({
    toast: {
        dismiss: vi.fn(),
        error: mocks.toastError,
        loading: vi.fn()
    }
}));

vi.mock('@/services/profileBackupService', () => ({
    getProfileBackupSettings: mocks.getSettings,
    runManualProfileBackup: mocks.runManual,
    setProfileBackupSettings: mocks.setSettings,
    validateProfileRestore: vi.fn()
}));

vi.mock('@/services/shellIntegrationService', () => ({
    openFileSelectorDialog: vi.fn(),
    openFolderSelectorDialog: mocks.selectFolder
}));

import { useProfileBackupStore } from '@/state/profileBackupStore';

import { useProfileBackupSettings } from './useProfileBackupSettings';

const initialSettings: ProfileBackupSettings = {
    autoEnabled: false,
    autoIntervalDays: 7,
    autoRetainExtra: 2,
    autoTargetDir: 'D:\\Backups',
    lastAutoAt: null
};

const acceptedOutcome: ProfileBackupActionOutcome = {
    accepted: true,
    error: null,
    status: {
        revision: 1,
        state: 'running',
        kind: 'manual',
        phase: 'snapshot',
        percent: 0,
        error: null,
        lastOutcome: null
    }
};

type HookValue = ReturnType<typeof useProfileBackupSettings>;

function deferred<T>() {
    let resolvePromise: ((value: T) => void) | null = null;
    const promise = new Promise<T>((resolve) => {
        resolvePromise = resolve;
    });
    return {
        promise,
        resolve(value: T) {
            if (!resolvePromise) {
                throw new Error('Deferred promise is unavailable.');
            }
            resolvePromise(value);
        }
    };
}

function HookHarness({ onValue }: { onValue: (value: HookValue) => void }) {
    onValue(useProfileBackupSettings(true));
    return null;
}

describe('useProfileBackupSettings', () => {
    let current: HookValue | null;

    beforeEach(() => {
        current = null;
        mocks.getSettings.mockReset().mockResolvedValue(initialSettings);
        mocks.runManual.mockReset().mockResolvedValue(acceptedOutcome);
        mocks.setSettings
            .mockReset()
            .mockImplementation(async (settings) => settings);
        mocks.selectFolder.mockReset().mockResolvedValue('E:\\Profile');
        mocks.toastError.mockReset();
        useProfileBackupStore.getState().resetProfileBackupState();
    });

    afterEach(cleanup);

    it('starts the manual backup after the folder is selected', async () => {
        render(<HookHarness onValue={(value) => (current = value)} />);

        await waitFor(() => {
            expect(current?.settings).toEqual(initialSettings);
        });
        await act(async () => {
            await current?.startManualBackup();
        });

        expect(mocks.runManual).toHaveBeenCalledWith('E:\\Profile');
        expect(mocks.selectFolder.mock.invocationCallOrder[0]).toBeLessThan(
            mocks.runManual.mock.invocationCallOrder[0]
        );
    });

    it('enables automatic backups without a confirmation', async () => {
        render(<HookHarness onValue={(value) => (current = value)} />);

        await waitFor(() => {
            expect(current?.settings).toEqual(initialSettings);
        });
        await act(async () => {
            await current?.setAutoEnabled(true);
        });

        expect(mocks.setSettings).toHaveBeenCalledWith({
            ...initialSettings,
            autoEnabled: true
        });
        expect(mocks.selectFolder).not.toHaveBeenCalled();
    });

    it('allows only one manual flow while folder selection is pending', async () => {
        const folderSelection = deferred<string>();
        mocks.selectFolder.mockReturnValue(folderSelection.promise);
        render(<HookHarness onValue={(value) => (current = value)} />);

        await waitFor(() => {
            expect(current?.settings).toEqual(initialSettings);
        });
        const value = current;
        if (!value) {
            throw new Error('Profile backup settings did not load.');
        }
        await act(async () => {
            const first = value.startManualBackup();
            const second = value.startManualBackup();
            folderSelection.resolve('E:\\Profile');
            await Promise.all([first, second]);
        });

        expect(mocks.selectFolder).toHaveBeenCalledTimes(1);
        expect(mocks.runManual).toHaveBeenCalledTimes(1);
    });

    it('reloads the last automatic backup time after an automatic success', async () => {
        const updatedSettings = {
            ...initialSettings,
            lastAutoAt: '2026-07-14T09:00:00Z'
        };
        mocks.getSettings
            .mockResolvedValueOnce(initialSettings)
            .mockResolvedValueOnce(updatedSettings);
        render(<HookHarness onValue={(value) => (current = value)} />);

        await waitFor(() => {
            expect(current?.settings).toEqual(initialSettings);
        });
        act(() => {
            useProfileBackupStore.getState().applyStatus({
                revision: 5,
                state: 'idle',
                kind: null,
                phase: null,
                percent: null,
                error: null,
                lastOutcome: {
                    revision: 5,
                    kind: 'auto',
                    succeeded: true,
                    fileName: 'VRCX-0-auto.vrcx0backup',
                    errorCode: null
                }
            });
        });

        await waitFor(() => {
            expect(current?.settings).toEqual(updatedSettings);
        });
        expect(mocks.getSettings).toHaveBeenCalledTimes(2);
    });

    it('does not let an earlier settings load overwrite an automatic refresh', async () => {
        const initialLoad = deferred<ProfileBackupSettings>();
        const automaticRefresh = deferred<ProfileBackupSettings>();
        const updatedSettings = {
            ...initialSettings,
            lastAutoAt: '2026-07-14T09:00:00Z'
        };
        mocks.getSettings
            .mockReturnValueOnce(initialLoad.promise)
            .mockReturnValueOnce(automaticRefresh.promise);
        render(<HookHarness onValue={(value) => (current = value)} />);

        await waitFor(() => {
            expect(mocks.getSettings).toHaveBeenCalledTimes(1);
        });
        act(() => {
            useProfileBackupStore.getState().applyStatus({
                revision: 6,
                state: 'idle',
                kind: null,
                phase: null,
                percent: null,
                error: null,
                lastOutcome: {
                    revision: 6,
                    kind: 'auto',
                    succeeded: true,
                    fileName: 'VRCX-0-auto.vrcx0backup',
                    errorCode: null
                }
            });
        });
        await waitFor(() => {
            expect(mocks.getSettings).toHaveBeenCalledTimes(2);
        });
        await act(async () => {
            automaticRefresh.resolve(updatedSettings);
            await automaticRefresh.promise;
        });
        await waitFor(() => {
            expect(current?.settings).toEqual(updatedSettings);
        });
        await act(async () => {
            initialLoad.resolve(initialSettings);
            await initialLoad.promise;
        });

        expect(current?.settings).toEqual(updatedSettings);
    });
});
