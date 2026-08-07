import { SettingsTabContent } from '../SettingsViewParts';
import { AssistantSettingsGroup } from './AssistantSettingsGroup';

type SettingsAiTabProps = {
    active: boolean;
};

export function SettingsAiTab({ active }: SettingsAiTabProps) {
    return (
        <SettingsTabContent value="ai">
            <AssistantSettingsGroup active={active} />
        </SettingsTabContent>
    );
}
