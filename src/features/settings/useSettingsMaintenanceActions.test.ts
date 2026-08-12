import { describe, expect, it, vi } from 'vitest';

import {
    DEFAULT_HMD_NOTIFICATION_ACTIVITY_FILTERS,
    DEFAULT_OVERLAY_ACTIVITY_FILTERS,
    DEFAULT_TTS_NOTIFICATION_ACTIVITY_FILTERS,
    DEFAULT_VR_NOTIFICATION_ACTIVITY_FILTERS,
    DEFAULT_WEBHOOK_ACTIVITY_FILTERS
} from './settingsValues';
import { createSettingsMaintenanceActions } from './useSettingsMaintenanceActions';

function createMaintenanceActions({
    cleanupAvatarFeedHistory = async () => ({
        deletedRows: 0,
        status: 'completed' as const,
        optimizationError: null
    }),
    confirm,
    isGameRunning = false,
    setGameLogPersistenceDisabledPreference = async () => undefined,
    setFeedPersistenceDisabledPreference = async () => undefined,
    setAvatarFeedPersistenceDisabledPreference = async () => undefined,
    setPurgeDialogOpen = () => undefined,
    toastWarning = () => undefined
}: {
    cleanupAvatarFeedHistory?: () => Promise<{
        deletedRows: number;
        status: 'completed' | 'optimizationFailed';
        optimizationError: string | null;
    }>;
    confirm: (options: {
        title: string;
        description: string;
    }) => Promise<{ ok: boolean }>;
    isGameRunning?: boolean;
    setGameLogPersistenceDisabledPreference?: (
        disabled: boolean
    ) => Promise<void>;
    setFeedPersistenceDisabledPreference?: (disabled: boolean) => Promise<void>;
    setAvatarFeedPersistenceDisabledPreference?: (
        disabled: boolean
    ) => Promise<void>;
    setPurgeDialogOpen?: (open: boolean) => void;
    toastWarning?: (message: string) => void;
}) {
    return createSettingsMaintenanceActions({
        alert: async () => ({ ok: true, reason: 'ok' }),
        avatarFeedHistoryRepository: {
            cleanupAvatarFeedHistory
        },
        commit: async () => true,
        confirm,
        gameState: {
            isGameRunning
        },
        mediaRepository: {
            cropAllPrints: async () => undefined,
            getUgcPhotoLocation: async () => ''
        },
        prefs: {
            appCjkFontPack: null,
            appFontFamily: null,
            autoLoginDelaySeconds: 0,
            customFontFamily: null,
            customFontOverride: null,
            customFontPrimary: null,
            customFontSecondary: null,
            desktopNotificationActivityFilters:
                DEFAULT_VR_NOTIFICATION_ACTIVITY_FILTERS,
            hmdNotificationActivityFilters:
                DEFAULT_HMD_NOTIFICATION_ACTIVITY_FILTERS,
            notificationTTS: 'Never',
            notificationTTSNameMode: '',
            notificationTTSVoiceNative: '',
            overlayActivityFilters: DEFAULT_OVERLAY_ACTIVITY_FILTERS,
            proxyServer: '',
            ttsNotificationActivityFilters:
                DEFAULT_TTS_NOTIFICATION_ACTIVITY_FILTERS,
            userGeneratedContentPath: '',
            vrNotificationActivityFilters:
                DEFAULT_VR_NOTIFICATION_ACTIVITY_FILTERS,
            webhookActivityFilters: DEFAULT_WEBHOOK_ACTIVITY_FILTERS,
            wristOverlayEnabled: false
        },
        prompt: async () => ({ ok: false }),
        purgePeriod: '180',
        savePreferenceValue: async (_key, _value, action) => {
            await action();
            return true;
        },
        saveStringPreference: async () => undefined,
        setAppDataDirState: () => undefined,
        setCropInstancePrintsPreference: async () => undefined,
        setGameLogPersistenceDisabledPreference,
        setFeedPersistenceDisabledPreference,
        setAvatarFeedPersistenceDisabledPreference,
        setIntConfigPreference: async () => undefined,
        setPrefs: () => undefined,
        setPurgeDialogOpen,
        setPurgeInProgress: () => undefined,
        setUserGeneratedContentPathPreference: async () => '',
        speakNotificationTts: async () => undefined,
        t: (key) => key,
        toast: {
            dismiss: () => undefined,
            error: () => undefined,
            success: () => undefined,
            warning: toastWarning
        }
    });
}

describe('handleGameLogDisabledChange', () => {
    it('keeps GameLog enabled when disabling is not confirmed', async () => {
        const confirm = vi.fn(async () => ({ ok: false }));
        const setGameLogPersistenceDisabledPreference = vi.fn(
            async () => undefined
        );
        const actions = createMaintenanceActions({
            confirm,
            setGameLogPersistenceDisabledPreference
        });

        await actions.handleGameLogDisabledChange(true);

        expect(confirm).toHaveBeenCalledOnce();
        expect(setGameLogPersistenceDisabledPreference).not.toHaveBeenCalled();
    });

    it('enables GameLog without showing the disable confirmation', async () => {
        const confirm = vi.fn(async () => ({ ok: false }));
        const setGameLogPersistenceDisabledPreference = vi.fn(
            async () => undefined
        );
        const actions = createMaintenanceActions({
            confirm,
            setGameLogPersistenceDisabledPreference
        });

        await actions.handleGameLogDisabledChange(false);

        expect(confirm).not.toHaveBeenCalled();
        expect(setGameLogPersistenceDisabledPreference).toHaveBeenCalledWith(
            false
        );
    });

    it('rejects changes while VRChat is running', async () => {
        const confirm = vi.fn(async () => ({ ok: true }));
        const setGameLogPersistenceDisabledPreference = vi.fn(
            async () => undefined
        );
        const actions = createMaintenanceActions({
            confirm,
            isGameRunning: true,
            setGameLogPersistenceDisabledPreference
        });

        await actions.handleGameLogDisabledChange(true);

        expect(confirm).not.toHaveBeenCalled();
        expect(setGameLogPersistenceDisabledPreference).not.toHaveBeenCalled();
    });
});

describe('handleFeedPersistenceDisabledChange', () => {
    it('keeps Feed history enabled when disabling is not confirmed', async () => {
        const setFeedPersistenceDisabledPreference = vi.fn(
            async () => undefined
        );
        const actions = createMaintenanceActions({
            confirm: async () => ({ ok: false }),
            setFeedPersistenceDisabledPreference
        });

        await actions.handleFeedPersistenceDisabledChange(true);

        expect(setFeedPersistenceDisabledPreference).not.toHaveBeenCalled();
    });

    it('can switch Feed persistence while VRChat is running', async () => {
        const setFeedPersistenceDisabledPreference = vi.fn(
            async () => undefined
        );
        const actions = createMaintenanceActions({
            confirm: async () => ({ ok: true }),
            isGameRunning: true,
            setFeedPersistenceDisabledPreference
        });

        await actions.handleFeedPersistenceDisabledChange(true);

        expect(setFeedPersistenceDisabledPreference).toHaveBeenCalledWith(true);
    });
});

describe('handleAvatarFeedPersistenceDisabledChange', () => {
    it('switches avatar Feed persistence without confirmation', async () => {
        const confirm = vi.fn(async () => ({ ok: false }));
        const setAvatarFeedPersistenceDisabledPreference = vi.fn(
            async () => undefined
        );
        const actions = createMaintenanceActions({
            confirm,
            setAvatarFeedPersistenceDisabledPreference
        });

        await actions.handleAvatarFeedPersistenceDisabledChange(true);

        expect(confirm).not.toHaveBeenCalled();
        expect(setAvatarFeedPersistenceDisabledPreference).toHaveBeenCalledWith(
            true
        );
    });
});

describe('purgeAvatarFeedData', () => {
    it('reports a completed purge separately from a failed optimization', async () => {
        const setPurgeDialogOpen = vi.fn();
        const toastWarning = vi.fn();
        const actions = createMaintenanceActions({
            cleanupAvatarFeedHistory: async () => ({
                deletedRows: 12,
                status: 'optimizationFailed',
                optimizationError: 'vacuum failed'
            }),
            confirm: async () => ({ ok: false }),
            setPurgeDialogOpen,
            toastWarning
        });

        await actions.purgeAvatarFeedData();

        expect(setPurgeDialogOpen).toHaveBeenCalledWith(false);
        expect(toastWarning).toHaveBeenCalledWith(
            'view.settings.advanced.advanced.database_cleanup.purge_optimization_failed'
        );
    });
});
