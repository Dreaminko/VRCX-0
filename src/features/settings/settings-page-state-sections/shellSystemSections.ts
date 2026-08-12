import { settingsTabs } from '../settingsOptions';
import type { BuildSettingsPageStateSectionsInput } from '../settingsPageStateSections';

export function buildShellSection({
    activeSettingsTab,
    setActiveSettingsTab
}: BuildSettingsPageStateSectionsInput) {
    return {
        activeSettingsTab,
        setActiveSettingsTab,
        settingsTabs
    };
}

export function buildSystemSection({
    savePreferenceValue,
    saveBoolPreference,
    setProxyEnabledPreference,
    setStartAtWindowsStartupPreference,
    setStartAsMinimizedPreference,
    setCloseToTrayPreference,
    setSystemWindowFramePreference,
    promptAutoLoginDelaySeconds,
    promptBackgroundModeDelayMinutes
}: BuildSettingsPageStateSectionsInput) {
    return {
        savePreferenceValue,
        saveBoolPreference,
        setProxyEnabledPreference,
        setStartAtWindowsStartupPreference,
        setStartAsMinimizedPreference,
        setCloseToTrayPreference,
        setSystemWindowFramePreference,
        promptAutoLoginDelaySeconds,
        promptBackgroundModeDelayMinutes
    };
}
