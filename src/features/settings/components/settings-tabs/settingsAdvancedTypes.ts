import type { AppDataDirState } from '@/platform/tauri/bindings';
import type { AvatarAutoCleanupPreference } from '@/shared/constants/settings';

export type SettingsAdvancedPrefs = {
    anonymousUsageTelemetry?: boolean;
    autoSweepVRChatCache?: boolean;
    avatarAutoCleanup?: AvatarAutoCleanupPreference;
    gameLogDisabled?: boolean;
    feedPersistenceDisabled?: boolean;
    avatarFeedPersistenceDisabled?: boolean;
    focusVrchatOnJoin?: boolean;
    logResourceLoad?: boolean;
    relaunchVRChatAfterCrash?: boolean;
    udonExceptionLogging?: boolean;
    vrcQuitFix?: boolean;
};

export type SettingsAdvancedAction = () => unknown | Promise<unknown>;

export type SettingsAdvancedModel = {
    appDataDirState?: AppDataDirState | null;
    hostPlatform?: string;
    avatarAutoCleanupOptions: readonly AvatarAutoCleanupPreference[];
    configTreeData: Record<string, unknown>;
    onAnonymousUsageTelemetryChange: (checked: boolean) => unknown;
    onAutoSweepVRChatCacheChange: (checked: boolean) => unknown;
    onAvatarAutoCleanupChange: (value: AvatarAutoCleanupPreference) => unknown;
    onClearConfigTreeData: () => void;
    onCleanupAppDataDir: SettingsAdvancedAction;
    onDismissAppDataDirCleanup: SettingsAdvancedAction;
    onGameLogDisabledChange: (disabled: boolean) => unknown;
    onFeedPersistenceDisabledChange: (disabled: boolean) => unknown;
    onAvatarFeedPersistenceDisabledChange: (disabled: boolean) => unknown;
    onFocusVrchatOnJoinChange: (checked: boolean) => unknown;
    onLogResourceLoadChange: (checked: boolean) => unknown;
    onMigrateLegacyVrcxData: SettingsAdvancedAction;
    onOpenAppDataDirSelector: SettingsAdvancedAction;
    onOpenPurgeDialog: () => void;
    onRefreshConfigTreeData: SettingsAdvancedAction;
    onRefreshOnlineVisits: SettingsAdvancedAction;
    onRefreshSqliteTableSizes: SettingsAdvancedAction;
    onRelaunchVRChatAfterCrashChange: (checked: boolean) => unknown;
    onResetAppDataDir: SettingsAdvancedAction;
    onUdonExceptionLoggingChange: (checked: boolean) => unknown;
    onVrcQuitFixChange: (checked: boolean) => unknown;
    onlineVisitCount: number | null;
    prefs: SettingsAdvancedPrefs;
    sqliteTableSizeRows: ReadonlyArray<readonly [string, string]>;
    sqliteTableSizes: Record<string, unknown>;
};
