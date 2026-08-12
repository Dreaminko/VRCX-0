import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { useShallow } from 'zustand/react/shallow';

import { POST_UPDATE_CHANGELOG_TOAST_CONFIG_KEY } from '@/services/changelogService';
import { restartApplication } from '@/services/shellIntegrationService';
import { isUpdateCheckDisabledBuild } from '@/shared/buildLabel';
import { usePreferencesStore } from '@/state/preferencesStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import { useSettingsPageSection } from '../SettingsPageStateContext';
import { normalizeCheckedState } from '../settingsValues';
import { SettingsSystemTab } from './settings-tabs/SettingsSystemTab';

export function SettingsSystemSection() {
    const { t } = useTranslation();
    const system = useSettingsPageSection('system');
    const hostPlatform = useRuntimeStore(
        (state) => state.hostCapabilities.platform
    );
    const setSystemHostOpen = useRuntimeStore(
        (state) => state.setSystemHostOpen
    );
    const prefs = usePreferencesStore(
        useShallow((state) => ({
            isStartAtWindowsStartup: state.isStartAtWindowsStartup,
            isStartAsMinimizedState: state.isStartAsMinimizedState,
            isCloseToTray: state.isCloseToTray,
            systemWindowFrame: state.systemWindowFrame,
            autoLoginDelayEnabled: state.autoLoginDelayEnabled,
            autoLoginDelaySeconds: state.autoLoginDelaySeconds,
            autoInstallUpdatesOnStartup: state.autoInstallUpdatesOnStartup,
            showPostUpdateChangelogToast: state.showPostUpdateChangelogToast,
            backgroundModeEnabled: state.backgroundModeEnabled,
            backgroundModeDelayEnabled: state.backgroundModeDelayEnabled,
            backgroundModeDelayMinutes: state.backgroundModeDelayMinutes,
            proxyEnabled: state.proxyEnabled,
            proxyServer: state.proxyServer
        }))
    );
    const {
        savePreferenceValue,
        saveBoolPreference,
        setProxyEnabledPreference,
        setStartAtWindowsStartupPreference,
        setStartAsMinimizedPreference,
        setCloseToTrayPreference,
        setSystemWindowFramePreference,
        promptAutoLoginDelaySeconds,
        promptBackgroundModeDelayMinutes
    } = system;

    return (
        <SettingsSystemTab
            hostPlatform={hostPlatform}
            isStartAtWindowsStartup={prefs.isStartAtWindowsStartup}
            isStartAsMinimizedState={prefs.isStartAsMinimizedState}
            isCloseToTray={prefs.isCloseToTray}
            systemWindowFrame={prefs.systemWindowFrame}
            autoLoginDelayEnabled={prefs.autoLoginDelayEnabled}
            autoLoginDelaySeconds={prefs.autoLoginDelaySeconds}
            autoInstallUpdatesOnStartup={prefs.autoInstallUpdatesOnStartup}
            updateCheckDisabled={isUpdateCheckDisabledBuild()}
            showPostUpdateChangelogToast={prefs.showPostUpdateChangelogToast}
            backgroundModeEnabled={prefs.backgroundModeEnabled}
            backgroundModeDelayEnabled={prefs.backgroundModeDelayEnabled}
            backgroundModeDelayMinutes={prefs.backgroundModeDelayMinutes}
            proxyEnabled={prefs.proxyEnabled}
            proxyServer={prefs.proxyServer}
            onStartAtWindowsStartupChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                savePreferenceValue('isStartAtWindowsStartup', enabled, () =>
                    setStartAtWindowsStartupPreference(enabled)
                );
            }}
            onStartAsMinimizedChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                savePreferenceValue('isStartAsMinimizedState', enabled, () =>
                    setStartAsMinimizedPreference(enabled)
                );
            }}
            onSystemWindowFrameChange={async (checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                const saved = await savePreferenceValue(
                    'systemWindowFrame',
                    enabled,
                    () => setSystemWindowFramePreference(enabled)
                );
                if (saved) {
                    toast(
                        t(
                            'view.settings.general.application.system_window_frame_saved'
                        ),
                        {
                            action: {
                                label: t(
                                    'view.settings.general.application.system_window_frame_restart_now'
                                ),
                                onClick: () => {
                                    void restartApplication();
                                }
                            }
                        }
                    );
                }
            }}
            onCloseToTrayChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                savePreferenceValue('isCloseToTray', enabled, () =>
                    setCloseToTrayPreference(enabled)
                );
            }}
            onAutoLoginDelayEnabledChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                saveBoolPreference(
                    'autoLoginDelayEnabled',
                    'autoLoginDelayEnabled',
                    enabled
                );
            }}
            onBackgroundModeEnabledChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                saveBoolPreference(
                    'backgroundModeEnabled',
                    'backgroundModeEnabled',
                    enabled
                );
            }}
            onBackgroundModeDelayEnabledChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                saveBoolPreference(
                    'backgroundModeDelayEnabled',
                    'backgroundModeDelayEnabled',
                    enabled
                );
            }}
            onAutoInstallUpdatesOnStartupChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                saveBoolPreference(
                    'autoInstallUpdatesOnStartup',
                    'autoInstallUpdatesOnStartup',
                    enabled
                );
            }}
            onPostUpdateChangelogToastChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                saveBoolPreference(
                    'showPostUpdateChangelogToast',
                    POST_UPDATE_CHANGELOG_TOAST_CONFIG_KEY,
                    enabled
                );
            }}
            onPromptAutoLoginDelaySeconds={() => {
                promptAutoLoginDelaySeconds();
            }}
            onPromptBackgroundModeDelayMinutes={() => {
                promptBackgroundModeDelayMinutes();
            }}
            onProxyEnabledChange={async (checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                const saved = await savePreferenceValue(
                    'proxyEnabled',
                    enabled,
                    () => setProxyEnabledPreference(enabled)
                );
                if (saved) {
                    toast.success(
                        t('prompt.proxy_settings.saved_restart_required')
                    );
                }
            }}
            onProxySettings={() => {
                setSystemHostOpen('proxySettingsOpen', true);
            }}
        />
    );
}
