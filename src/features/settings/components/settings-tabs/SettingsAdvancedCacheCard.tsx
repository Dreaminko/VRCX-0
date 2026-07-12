import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/utils';
import { Button } from '@/ui/shadcn/button';

import { Field } from '../SettingsField';

export function SettingsAdvancedCacheCard({
    autoSweepEnabled,
    onPromptAutoClearVrcxCacheFrequency
}: any) {
    const { t } = useTranslation();
    return (
        <>
            <Field
                label={t(
                    'view.settings.advanced_groups.diagnostics_maintenance.configure_auto_clear'
                )}
                className={cn(
                    !autoSweepEnabled && 'pointer-events-none opacity-50'
                )}
            >
                <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={onPromptAutoClearVrcxCacheFrequency}
                >
                    {t(
                        'view.settings.advanced_groups.diagnostics_maintenance.configure_auto_clear'
                    )}
                </Button>
            </Field>
        </>
    );
}
