import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { RuntimeModelSelect } from '@/features/assistant/components/RuntimeModelSelect';
import {
    commands,
    type AssistantRuntimeSelection,
    type PlaybookMode
} from '@/platform/tauri/bindings';
import {
    openLlmEndpointsManager,
    useLlmEndpointsStore
} from '@/state/llmEndpointsStore';
import { Button } from '@/ui/shadcn/button';
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';
import { Switch } from '@/ui/shadcn/switch';

import { Field, SettingsGroup } from '../SettingsField';

const PLAYBOOK_MODES: PlaybookMode[] = ['auto', 'guided', 'open'];

const EMPTY_SELECTION: AssistantRuntimeSelection = {
    endpointId: null,
    model: null,
    allowWrites: false,
    playbookMode: 'auto'
};

function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

export function AssistantSettingsGroup() {
    const { t } = useTranslation();
    const loadEndpoints = useLlmEndpointsStore((state) => state.load);
    const [selection, setSelection] =
        useState<AssistantRuntimeSelection>(EMPTY_SELECTION);
    const [followCustomProxy, setFollowCustomProxy] = useState(true);
    const [proxyLoading, setProxyLoading] = useState(false);

    useEffect(() => {
        loadEndpoints().catch(() => {});
        commands
            .appAssistantRuntimeStatus()
            .then((status) => setSelection(status.lastSelection))
            .catch(() => {});
        commands
            .appLlmEndpointFollowCustomProxy()
            .then(setFollowCustomProxy)
            .catch(() => {});
    }, [loadEndpoints]);

    async function updateSelection(patch: Partial<AssistantRuntimeSelection>) {
        const next = { ...selection, ...patch };
        setSelection(next);
        try {
            setSelection(
                await commands.appAssistantSetDefaultRuntime(
                    next.endpointId,
                    next.model,
                    next.allowWrites,
                    next.playbookMode
                )
            );
        } catch (error) {
            toast.error(errorMessage(error));
        }
    }

    async function updateFollowCustomProxy(enabled: boolean) {
        setProxyLoading(true);
        try {
            setFollowCustomProxy(
                await commands.appLlmEndpointSetFollowCustomProxy(enabled)
            );
        } catch (error) {
            toast.error(errorMessage(error));
        } finally {
            setProxyLoading(false);
        }
    }

    const playbookItems = PLAYBOOK_MODES.map((mode) => ({
        value: mode,
        label: t(`assistant.settings.playbook_mode_${mode}`)
    }));

    return (
        <SettingsGroup
            title={t('view.settings.ai.header')}
            description={t('view.settings.ai.description')}
        >
            <Field
                label={t('view.settings.ai.default_model')}
                description={t('view.settings.ai.default_model_description')}
            >
                <RuntimeModelSelect
                    endpointId={selection.endpointId}
                    model={selection.model}
                    placeholder={t('view.settings.ai.default_model_unset')}
                    onSelect={(ref) => void updateSelection(ref)}
                />
            </Field>

            <Field
                label={t('assistant.runtime.playbook_mode')}
                description={t('view.settings.ai.playbook_mode_description')}
            >
                <Select
                    value={selection.playbookMode}
                    items={playbookItems}
                    onValueChange={(value) =>
                        void updateSelection({
                            playbookMode: value ?? 'auto'
                        })
                    }
                >
                    <SelectTrigger className="w-full">
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectGroup>
                            {playbookItems.map((item) => (
                                <SelectItem key={item.value} value={item.value}>
                                    {item.label}
                                </SelectItem>
                            ))}
                        </SelectGroup>
                    </SelectContent>
                </Select>
            </Field>

            <Field
                label={t('view.tools.llm_endpoints.title')}
                description={t('view.tools.llm_endpoints.description')}
            >
                <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={openLlmEndpointsManager}
                >
                    {t('assistant.runtime.manage_endpoints')}
                </Button>
            </Field>

            <Field
                label={t('view.tools.llm_endpoints.follow_custom_proxy')}
                description={t(
                    'view.tools.llm_endpoints.follow_custom_proxy_description'
                )}
            >
                <Switch
                    checked={followCustomProxy}
                    disabled={proxyLoading}
                    onCheckedChange={updateFollowCustomProxy}
                />
            </Field>
        </SettingsGroup>
    );
}
