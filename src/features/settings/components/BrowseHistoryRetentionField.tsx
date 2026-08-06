import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { browseHistoryRepository } from '@/repositories/browseHistoryRepository';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';

import { Field } from './SettingsField';

const RETENTION_OPTIONS = [-1, 7, 30, 90, 365, 0] as const;

function isRetentionOption(value: number) {
    return RETENTION_OPTIONS.some((option) => option === value);
}

export function BrowseHistoryRetentionField() {
    const { t } = useTranslation();
    const [retentionDays, setRetentionDays] = useState<number | null>(null);

    function retentionLabel(days: number) {
        if (days === -1) {
            return t('browse_history.retention.off');
        }
        if (days === 0) {
            return t('browse_history.retention.forever');
        }
        return t('browse_history.retention.days', { count: days });
    }

    useEffect(() => {
        let active = true;
        void browseHistoryRepository
            .getRetentionDays()
            .then((days) => {
                if (active) {
                    setRetentionDays(days);
                }
            })
            .catch(() => undefined);
        return () => {
            active = false;
        };
    }, []);

    if (retentionDays === null) {
        return null;
    }

    function changeRetention(value: unknown) {
        const next = Number(value);
        if (!isRetentionOption(next)) {
            return;
        }
        const previous = retentionDays;
        setRetentionDays(next);
        void browseHistoryRepository.setRetentionDays(next).catch(() => {
            setRetentionDays(previous);
            toast.error(t('browse_history.retention.update_failed'));
        });
    }

    return (
        <Field
            label={t('browse_history.retention.label')}
            description={t('browse_history.retention.description')}
        >
            <Select
                value={String(retentionDays)}
                onValueChange={changeRetention}
            >
                <SelectTrigger
                    size="sm"
                    aria-label={t('browse_history.retention.label')}
                >
                    <SelectValue>
                        {(value: string) => retentionLabel(Number(value))}
                    </SelectValue>
                </SelectTrigger>
                <SelectContent align="end">
                    {RETENTION_OPTIONS.map((days) => (
                        <SelectItem key={days} value={String(days)}>
                            {retentionLabel(days)}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
        </Field>
    );
}
