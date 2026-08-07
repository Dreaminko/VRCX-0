import { SettingsTabContent } from '../SettingsViewParts';
import { AssistantSettingsGroup } from './AssistantSettingsGroup';

export function SettingsAiTab() {
    return (
        <SettingsTabContent value="ai">
            <AssistantSettingsGroup />
        </SettingsTabContent>
    );
}
