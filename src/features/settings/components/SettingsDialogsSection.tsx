import type { SettingsPageStateSections } from '../settingsPageStateSections';
import { SettingsDialogs } from './SettingsDialogs';

type SettingsDialogsSectionProps = {
    dialogs: SettingsPageStateSections['dialogs'];
};

export function SettingsDialogsSection({
    dialogs
}: SettingsDialogsSectionProps) {
    return <SettingsDialogs dialogs={dialogs} />;
}
