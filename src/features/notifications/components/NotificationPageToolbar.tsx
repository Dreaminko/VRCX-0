import { CheckCheckIcon, RefreshCcwIcon } from 'lucide-react';
import type { ChangeEvent } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/ui/shadcn/button';
import { Input } from '@/ui/shadcn/input';
import { Spinner } from '@/ui/shadcn/spinner';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/ui/shadcn/tooltip';

import type { NotificationLoadStatus } from '../notificationPageTypes';
import type { NotificationQuickFilter } from '../useNotificationFilters';
import { NotificationTypeFilterDropdown } from './NotificationViewParts';

const QUICK_FILTERS: { value: NotificationQuickFilter; labelKey: string }[] = [
    { value: 'all', labelKey: 'view.notification.feed.all' },
    {
        value: 'action',
        labelKey: 'side_panel.notification_center.group_action'
    },
    { value: 'unread', labelKey: 'view.notification.feed.unread' }
];

type NotificationPageToolbarProps = {
    activeTypes: string[];
    loadStatus: NotificationLoadStatus;
    notificationTypeLabel: (type: unknown) => string;
    onActiveTypesChange: (types: string[]) => void;
    onClearFilters: () => void;
    onMarkAllSeen: () => void;
    onQuickFilterChange: (value: NotificationQuickFilter) => void;
    onRefresh: () => void;
    onSearchQueryChange: (value: string) => void;
    quickFilter: NotificationQuickFilter;
    searchQuery: string;
    unseenCount: number;
};

export function NotificationPageToolbar({
    activeTypes,
    searchQuery,
    notificationTypeLabel,
    loadStatus,
    quickFilter,
    unseenCount,
    onActiveTypesChange,
    onSearchQueryChange,
    onQuickFilterChange,
    onMarkAllSeen,
    onRefresh,
    onClearFilters
}: NotificationPageToolbarProps) {
    const { t } = useTranslation();
    const refreshLabel = t('view.notification.refresh_tooltip');
    const markAllSeenLabel = t('side_panel.notification_center.mark_all_read');

    return (
        <div className="flex flex-wrap items-center gap-2">
            {QUICK_FILTERS.map((entry) => (
                <Button
                    key={entry.value}
                    type="button"
                    size="sm"
                    variant={
                        quickFilter === entry.value ? 'secondary' : 'ghost'
                    }
                    onClick={() => onQuickFilterChange(entry.value)}
                >
                    {t(entry.labelKey)}
                </Button>
            ))}
            <NotificationTypeFilterDropdown
                value={activeTypes}
                onChange={onActiveTypesChange}
                getTypeLabel={notificationTypeLabel}
            />
            <Input
                value={searchQuery}
                onChange={(event: ChangeEvent<HTMLInputElement>) =>
                    onSearchQueryChange(event.target.value)
                }
                placeholder={t('common.actions.search')}
                className="h-9 min-w-36 flex-1 sm:max-w-52"
            />
            <Tooltip>
                <TooltipTrigger
                    render={
                        <Button
                            type="button"
                            variant="ghost"
                            size="icon-sm"
                            aria-label={markAllSeenLabel}
                            className="rounded-full"
                            disabled={unseenCount <= 0}
                            onClick={onMarkAllSeen}
                        >
                            <CheckCheckIcon data-icon="inline-start" />
                        </Button>
                    }
                />
                <TooltipContent>{markAllSeenLabel}</TooltipContent>
            </Tooltip>
            <Tooltip>
                <TooltipTrigger
                    render={
                        <Button
                            type="button"
                            variant="ghost"
                            size="icon-sm"
                            aria-label={refreshLabel}
                            className="rounded-full"
                            disabled={loadStatus === 'running'}
                            onClick={onRefresh}
                        >
                            {loadStatus === 'running' ? (
                                <Spinner data-icon="inline-start" />
                            ) : (
                                <RefreshCcwIcon data-icon="inline-start" />
                            )}
                        </Button>
                    }
                />
                <TooltipContent>{refreshLabel}</TooltipContent>
            </Tooltip>
            {activeTypes.length || quickFilter !== 'all' ? (
                <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={onClearFilters}
                >
                    {t('common.actions.clear')}
                </Button>
            ) : null}
        </div>
    );
}
