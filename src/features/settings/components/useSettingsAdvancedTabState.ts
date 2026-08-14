import { useShallow } from 'zustand/react/shallow';

import type { AvatarAutoCleanupPreference } from '@/shared/constants/settings';
import { usePreferencesStore } from '@/state/preferencesStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import { useSettingsPageSection } from '../SettingsPageStateContext';
import { normalizeCheckedState } from '../settingsValues';

export function useSettingsAdvancedTabState() {
    const advanced = useSettingsPageSection('advanced');
    const prefs = usePreferencesStore(
        useShallow((state) => ({
            relaunchVRChatAfterCrash: state.relaunchVRChatAfterCrash,
            vrcQuitFix: state.vrcQuitFix,
            focusVrchatOnJoin: state.focusVrchatOnJoin,
            autoSweepVRChatCache: state.autoSweepVRChatCache,
            avatarAutoCleanup: state.avatarAutoCleanup,
            gameLogDisabled: state.gameLogDisabled,
            feedPersistenceDisabled: state.feedPersistenceDisabled,
            avatarFeedPersistenceDisabled: state.avatarFeedPersistenceDisabled,
            anonymousUsageTelemetry: state.anonymousUsageTelemetry,
            udonExceptionLogging: state.udonExceptionLogging,
            logResourceLoad: state.logResourceLoad
        }))
    );
    const hostPlatform = useRuntimeStore(
        (state) => state.hostCapabilities.platform
    );
    const {
        avatarAutoCleanupOptions,
        sqliteTableSizes,
        sqliteTableSizeRows,
        onlineVisitCount,
        configTreeData,
        appDataDirState,
        saveBoolPreference,
        handleGameLogDisabledChange,
        handleFeedPersistenceDisabledChange,
        handleAvatarFeedPersistenceDisabledChange,
        saveStringPreference,
        setPurgeDialogOpen,
        refreshSqliteTableSizes,
        refreshOnlineVisits,
        refreshConfigTreeData,
        openAppDataDirSelector,
        resetAppDataDir,
        cleanupAppDataDir,
        dismissAppDataDirCleanup,
        setConfigTreeData,
        migrateLegacyVrcxData
    } = advanced;

    const advancedTab = {
        hostPlatform,
        prefs,
        avatarAutoCleanupOptions,
        sqliteTableSizes,
        sqliteTableSizeRows,
        onlineVisitCount,
        configTreeData,
        appDataDirState,
        onRelaunchVRChatAfterCrashChange: (checked: unknown) => {
            const enabled = normalizeCheckedState(checked);
            saveBoolPreference(
                'relaunchVRChatAfterCrash',
                'VRCX_relaunchVRChatAfterCrash',
                enabled
            );
        },
        onVrcQuitFixChange: (checked: unknown) => {
            saveBoolPreference(
                'vrcQuitFix',
                'vrcQuitFix',
                normalizeCheckedState(checked)
            );
        },
        onFocusVrchatOnJoinChange: (checked: unknown) => {
            saveBoolPreference(
                'focusVrchatOnJoin',
                'focusVrchatOnJoin',
                normalizeCheckedState(checked)
            );
        },
        onAutoSweepVRChatCacheChange: (checked: unknown) => {
            const enabled = normalizeCheckedState(checked);
            saveBoolPreference(
                'autoSweepVRChatCache',
                'VRCX_autoSweepVRChatCache',
                enabled
            );
        },
        onUdonExceptionLoggingChange: (checked: unknown) => {
            const enabled = normalizeCheckedState(checked);
            saveBoolPreference(
                'udonExceptionLogging',
                'VRCX_udonExceptionLogging',
                enabled
            );
        },
        onLogResourceLoadChange: (checked: unknown) => {
            saveBoolPreference(
                'logResourceLoad',
                'logResourceLoad',
                normalizeCheckedState(checked)
            );
        },
        onAnonymousUsageTelemetryChange: (checked: unknown) => {
            const enabled = normalizeCheckedState(checked);
            saveBoolPreference(
                'anonymousUsageTelemetry',
                'anonymousUsageTelemetry',
                enabled
            );
        },
        onGameLogDisabledChange: (checked: unknown) => {
            handleGameLogDisabledChange(normalizeCheckedState(checked));
        },
        onFeedPersistenceDisabledChange: (checked: unknown) => {
            handleFeedPersistenceDisabledChange(normalizeCheckedState(checked));
        },
        onAvatarFeedPersistenceDisabledChange: (checked: unknown) => {
            handleAvatarFeedPersistenceDisabledChange(
                normalizeCheckedState(checked)
            );
        },
        onAvatarAutoCleanupChange: (value: AvatarAutoCleanupPreference) => {
            saveStringPreference(
                'avatarAutoCleanup',
                'avatarAutoCleanup',
                value
            );
        },
        onOpenPurgeDialog: () => setPurgeDialogOpen(true),
        onMigrateLegacyVrcxData: migrateLegacyVrcxData,
        onRefreshSqliteTableSizes: refreshSqliteTableSizes,
        onRefreshOnlineVisits: refreshOnlineVisits,
        onRefreshConfigTreeData: refreshConfigTreeData,
        onOpenAppDataDirSelector: openAppDataDirSelector,
        onResetAppDataDir: resetAppDataDir,
        onCleanupAppDataDir: cleanupAppDataDir,
        onDismissAppDataDirCleanup: dismissAppDataDirCleanup,
        onClearConfigTreeData: () => setConfigTreeData({})
    };

    return advancedTab;
}
