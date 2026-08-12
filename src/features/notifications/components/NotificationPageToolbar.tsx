import { CheckCheckIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarIconButton,
    ToolbarRefreshButton,
    ToolbarSearch,
    ToolbarSegmented,
    ToolbarViews,
    type ToolbarSegmentOption
} from '@/components/layout/ToolbarControls';
import { Button } from '@/ui/shadcn/button';

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
    const quickFilterOptions: ToolbarSegmentOption<NotificationQuickFilter>[] =
        QUICK_FILTERS.map((entry) => ({
            value: entry.value,
            label: t(entry.labelKey)
        }));
    const hasActiveFilters =
        activeTypes.length > 0 ||
        quickFilter !== 'all' ||
        Boolean(searchQuery.trim());

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    <ToolbarSegmented
                        value={quickFilter}
                        onValueChange={onQuickFilterChange}
                        options={quickFilterOptions}
                    />
                    <NotificationTypeFilterDropdown
                        value={activeTypes}
                        onChange={onActiveTypesChange}
                        getTypeLabel={notificationTypeLabel}
                    />
                    {hasActiveFilters ? (
                        <Button
                            type="button"
                            variant="ghost"
                            onClick={onClearFilters}
                        >
                            {t('common.actions.clear')}
                        </Button>
                    ) : null}
                </ToolbarViews>

                <ToolbarSearch
                    value={searchQuery}
                    onValueChange={onSearchQueryChange}
                />

                <ToolbarActions>
                    <ToolbarIconButton
                        icon={CheckCheckIcon}
                        label={t(
                            'side_panel.notification_center.mark_all_read'
                        )}
                        disabled={unseenCount <= 0}
                        onClick={onMarkAllSeen}
                    />
                    <ToolbarRefreshButton
                        onRefresh={onRefresh}
                        loading={loadStatus === 'running'}
                        label={t('view.notification.refresh_tooltip')}
                    />
                </ToolbarActions>
            </PageToolbarRow>
        </PageToolbar>
    );
}
