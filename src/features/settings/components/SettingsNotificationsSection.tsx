import { useTranslation } from 'react-i18next';

import { POST_UPDATE_CHANGELOG_TOAST_CONFIG_KEY } from '@/services/changelogService';

import type { SettingsPageStateSections } from '../settingsPageStateSections';
import { normalizeCheckedState } from '../settingsValues';
import { SettingsNotificationsTab } from './settings-tabs/SettingsNotificationsTab';

type SettingsNotificationsSectionProps = {
    notifications: SettingsPageStateSections['notifications'];
};

export function SettingsNotificationsSection({
    notifications
}: SettingsNotificationsSectionProps) {
    const { t } = useTranslation();
    const {
        prefs,
        notificationLayoutOptions,
        desktopToastOptions,
        notificationTtsOptions,
        notificationTtsNameModeOptions,
        ttsVoices,
        notificationTtsTestVisible,
        notificationTtsTest,
        commit,
        setNotificationLayoutPreference,
        setPrefs,
        setDesktopNotificationsDialogOpen,
        setTtsNotificationsDialogOpen,
        saveStringPreference,
        saveBoolPreference,
        saveNotificationTtsMode,
        saveNotificationTtsVoice,
        setNotificationTtsTestVisible,
        setNotificationTtsTest,
        speakNotificationTts
    } = notifications;

    return (
        <SettingsNotificationsTab
            prefs={prefs}
            notificationLayoutOptions={notificationLayoutOptions}
            desktopToastOptions={desktopToastOptions}
            notificationTtsOptions={notificationTtsOptions}
            notificationTtsNameModeOptions={notificationTtsNameModeOptions}
            ttsVoices={ttsVoices}
            notificationTtsTestVisible={notificationTtsTestVisible}
            notificationTtsTest={notificationTtsTest}
            onNotificationLayoutChange={(value: string) => {
                commit(
                    async () => {
                        const nextLayout =
                            await setNotificationLayoutPreference(value);
                        setPrefs((current) => ({
                            ...current,
                            notificationLayout: nextLayout
                        }));
                    },
                    () => {
                        const previous = prefs.notificationLayout;
                        setPrefs((current) => ({
                            ...current,
                            notificationLayout: value
                        }));
                        return () =>
                            setPrefs((current) => ({
                                ...current,
                                notificationLayout: previous
                            }));
                    }
                );
            }}
            onNotificationIconDotChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                saveBoolPreference(
                    'notificationIconDot',
                    'notificationIconDot',
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
            onOpenDesktopNotificationFiltersDialog={() =>
                setDesktopNotificationsDialogOpen(true)
            }
            onOpenTtsNotificationFiltersDialog={() =>
                setTtsNotificationsDialogOpen(true)
            }
            onDesktopToastChange={(value: string) => {
                saveStringPreference('desktopToast', 'desktopToast', value);
            }}
            onAfkDesktopToastChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                saveBoolPreference(
                    'afkDesktopToast',
                    'afkDesktopToast',
                    enabled
                );
            }}
            onDesktopNotificationSoundChange={(checked: unknown) => {
                const enabled = normalizeCheckedState(checked);
                saveBoolPreference(
                    'desktopNotificationSound',
                    'desktopNotificationSound',
                    enabled
                );
            }}
            onNotificationTtsModeChange={(value: string) => {
                saveNotificationTtsMode(value);
            }}
            onNotificationTtsVoiceChange={(value: string) => {
                saveNotificationTtsVoice(value);
            }}
            onNotificationTtsNameModeChange={(value: string) => {
                saveStringPreference(
                    'notificationTTSNameMode',
                    'notificationTTSNameMode',
                    value
                );
            }}
            onNotificationTtsTestVisibleChange={setNotificationTtsTestVisible}
            onNotificationTtsTestChange={setNotificationTtsTest}
            onSpeakNotificationTts={(message: unknown) =>
                speakNotificationTts(
                    String(
                        message ||
                            t(
                                'view.settings.notifications.notifications.text_to_speech.tts_test_placeholder'
                            )
                    )
                )
            }
        />
    );
}
