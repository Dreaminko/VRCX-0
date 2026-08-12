import { useShallow } from 'zustand/react/shallow';

import { usePreferencesStore } from '@/state/preferencesStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import { useSettingsPageSection } from '../SettingsPageStateContext';
import { normalizeCheckedState } from '../settingsValues';
import { SettingsAdvancedTab } from './settings-tabs/SettingsAdvancedTab';

export function SettingsAdvancedSection() {
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
        onAvatarAutoCleanupChange: (value: string) => {
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

    return <SettingsAdvancedTab advanced={advancedTab} />;
}
