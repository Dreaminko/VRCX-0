import { useTranslation } from 'react-i18next';

import type { SettingsPageStateSections } from '../settingsPageStateSections';
import { normalizeCheckedState } from '../settingsValues';
import { SettingsAdvancedTab } from './settings-tabs/SettingsAdvancedTab';

type SettingsAdvancedSectionProps = {
    advanced: SettingsPageStateSections['advanced'];
};

export function SettingsAdvancedSection({
    advanced
}: SettingsAdvancedSectionProps) {
    const { t } = useTranslation();
    const {
        prefs,
        avatarAutoCleanupOptions,
        sqliteTableSizes,
        sqliteTableSizeRows,
        onlineVisitCount,
        configTreeData,
        appDataDirState,
        saveBoolPreference,
        promptAutoClearVrcxCacheFrequency,
        handleGameLogDisabledChange,
        saveStringPreference,
        setPurgeDialogOpen,
        refreshSqliteTableSizes,
        refreshOnlineVisits,
        refreshConfigTreeData,
        openAppDataDirSelector,
        resetAppDataDir,
        restartForAppDataDir,
        setConfigTreeData,
        migrateLegacyVrcxData
    } = advanced;

    const advancedTab = {
        prefs,
        avatarAutoCleanupOptions,
        sqliteTableSizes,
        sqliteTableSizeRows,
        onlineVisitCount,
        configTreeData,
        appDataDirState,
        gameLogDisabledLabel: t(
            'view.settings.advanced.advanced.cache_debug.disable_gamelog'
        ),
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
        onDefaultLaunchModeChange: (value: string) => {
            saveStringPreference(
                'defaultLaunchMode',
                'defaultLaunchMode',
                value
            );
        },
        onShowConfirmationOnSwitchAvatarChange: (checked: unknown) => {
            const enabled = normalizeCheckedState(checked);
            saveBoolPreference(
                'showConfirmationOnSwitchAvatar',
                'showConfirmationOnSwitchAvatar',
                enabled
            );
        },
        onPromptAutoClearVrcxCacheFrequency: () => {
            promptAutoClearVrcxCacheFrequency();
        },
        onGameLogDisabledChange: (checked: unknown) => {
            handleGameLogDisabledChange(normalizeCheckedState(checked));
        },
        onAvatarAutoCleanupChange: (value: string) => {
            saveStringPreference(
                'avatarAutoCleanup',
                'avatarAutoCleanup',
                value
            );
        },
        onOpenPurgeDialog: () => setPurgeDialogOpen(true),
        onMigrateLegacyVrcxData: () => {
            migrateLegacyVrcxData();
        },
        onRefreshSqliteTableSizes: () => {
            refreshSqliteTableSizes();
        },
        onRefreshOnlineVisits: () => {
            refreshOnlineVisits();
        },
        onRefreshConfigTreeData: () => {
            refreshConfigTreeData();
        },
        onOpenAppDataDirSelector: () => {
            openAppDataDirSelector();
        },
        onResetAppDataDir: () => {
            resetAppDataDir();
        },
        onRestartForAppDataDir: () => {
            restartForAppDataDir();
        },
        onClearConfigTreeData: () => setConfigTreeData({})
    };

    return <SettingsAdvancedTab advanced={advancedTab} />;
}
