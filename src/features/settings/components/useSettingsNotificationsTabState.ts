import { useTranslation } from 'react-i18next';
import { useShallow } from 'zustand/react/shallow';

import { usePreferencesStore } from '@/state/preferencesStore';

import { useSettingsPageSection } from '../SettingsPageStateContext';
import { normalizeCheckedState } from '../settingsValues';

export function useSettingsNotificationsTabState() {
    const { t } = useTranslation();
    const notifications = useSettingsPageSection('notifications');
    const prefs = usePreferencesStore(
        useShallow((state) => ({
            desktopToast: state.desktopToast,
            afkDesktopToast: state.afkDesktopToast,
            desktopNotificationSound: state.desktopNotificationSound,
            notificationTTS: state.notificationTTS,
            notificationTTSVoiceNative: state.notificationTTSVoiceNative,
            notificationTTSNameMode: state.notificationTTSNameMode,
            notificationTTSNickName: state.notificationTTSNickName
        }))
    );
    const {
        desktopToastOptions,
        notificationTtsOptions,
        notificationTtsNameModeOptions,
        ttsVoices,
        notificationTtsTestVisible,
        notificationTtsTest,
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

    return {
        prefs,
        desktopToastOptions,
        notificationTtsOptions,
        notificationTtsNameModeOptions,
        ttsVoices,
        notificationTtsTestVisible,
        notificationTtsTest,
        onOpenDesktopNotificationFiltersDialog: () =>
            setDesktopNotificationsDialogOpen(true),
        onOpenTtsNotificationFiltersDialog: () =>
            setTtsNotificationsDialogOpen(true),
        onDesktopToastChange: (value: string) => {
            saveStringPreference('desktopToast', 'desktopToast', value);
        },
        onAfkDesktopToastChange: (checked: unknown) => {
            const enabled = normalizeCheckedState(checked);
            saveBoolPreference('afkDesktopToast', 'afkDesktopToast', enabled);
        },
        onDesktopNotificationSoundChange: (checked: unknown) => {
            const enabled = normalizeCheckedState(checked);
            saveBoolPreference(
                'desktopNotificationSound',
                'desktopNotificationSound',
                enabled
            );
        },
        onNotificationTtsModeChange: (value: string) => {
            saveNotificationTtsMode(value);
        },
        onNotificationTtsVoiceChange: (value: string) => {
            saveNotificationTtsVoice(value);
        },
        onNotificationTtsNameModeChange: (value: string) => {
            saveStringPreference(
                'notificationTTSNameMode',
                'notificationTTSNameMode',
                value
            );
        },
        onNotificationTtsTestVisibleChange: setNotificationTtsTestVisible,
        onNotificationTtsTestChange: setNotificationTtsTest,
        onSpeakNotificationTts: (message: unknown) =>
            speakNotificationTts(
                String(
                    message ||
                        t(
                            'view.settings.notifications.notifications.text_to_speech.tts_test_placeholder'
                        )
                )
            )
    };
}
