import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

import type { AppTable } from '@/components/data-table/appTable';
import {
    DataTablePagination,
    DataTableSurface
} from '@/components/data-table/DataTableView';
import { EmptyState, LoadingState } from '@/components/layout/PageScaffold';
import { formatDateFilter } from '@/lib/dateTime';
import { userFacingErrorMessage } from '@/lib/errorDisplay';
import { getNotificationTs } from '@/shared/utils/notificationCategory';

import type {
    NotificationLoadStatus,
    NotificationRow as NotificationRecord
} from '../notificationPageTypes';
import {
    NotificationRow,
    type NotificationFeedHandlers
} from './NotificationRow';

type NotificationFeedDay = {
    key: string;
    timestamp: number;
    rows: NotificationRecord[];
};

function dayKey(timestamp: number): string {
    const date = new Date(timestamp);
    return [
        date.getFullYear(),
        String(date.getMonth() + 1).padStart(2, '0'),
        String(date.getDate()).padStart(2, '0')
    ].join('-');
}

function groupByDay(rows: NotificationRecord[]): NotificationFeedDay[] {
    const days: NotificationFeedDay[] = [];
    for (const row of rows) {
        const timestamp = getNotificationTs(row);
        const key = dayKey(timestamp);
        const current = days.at(-1);
        if (current?.key === key) {
            current.rows.push(row);
            continue;
        }
        days.push({ key, timestamp, rows: [row] });
    }
    return days;
}

export function NotificationFeed({
    rows,
    table,
    detail,
    loadStatus,
    rowsCount,
    pagination,
    pageSizes,
    onPageSizeChange,
    currentUserId,
    canInviteFromCurrentLocation,
    handlers
}: {
    canInviteFromCurrentLocation: boolean;
    currentUserId?: string;
    detail: string;
    handlers: NotificationFeedHandlers;
    loadStatus: NotificationLoadStatus;
    onPageSizeChange: (value: string) => void;
    pageSizes: number[];
    pagination: { pageIndex: number; pageSize: number };
    rows: NotificationRecord[];
    rowsCount: number;
    table: AppTable<NotificationRecord>;
}) {
    const { t } = useTranslation();
    const days = useMemo(() => groupByDay(rows), [rows]);
    const todayKey = dayKey(Date.now());
    const yesterdayKey = dayKey(Date.now() - 86_400_000);

    function dayLabel(day: NotificationFeedDay) {
        if (day.key === todayKey) {
            return t('view.notification.feed.today');
        }
        if (day.key === yesterdayKey) {
            return t('view.notification.feed.yesterday');
        }
        return formatDateFilter(day.timestamp, 'date');
    }

    return (
        <>
            {detail ? (
                <div className="text-muted-foreground text-sm">
                    {userFacingErrorMessage(
                        detail,
                        t(
                            'view.notifications.toast.failed_to_load_notifications'
                        )
                    )}
                </div>
            ) : null}

            <DataTableSurface>
                <div className="h-full min-h-0 min-w-0 overflow-auto px-2 pb-2">
                    {days.length > 0 ? (
                        days.map((day) => (
                            <div key={day.key}>
                                <div className="bg-background/95 text-muted-foreground/70 sticky top-0 z-10 px-2 pt-5 pb-2 text-xs font-medium">
                                    {dayLabel(day)}
                                </div>
                                {day.rows.map((notification) => (
                                    <NotificationRow
                                        key={String(notification.id)}
                                        notification={notification}
                                        currentUserId={currentUserId}
                                        canInviteFromCurrentLocation={
                                            canInviteFromCurrentLocation
                                        }
                                        handlers={handlers}
                                    />
                                ))}
                            </div>
                        ))
                    ) : loadStatus === 'running' ? (
                        <LoadingState
                            variant="table"
                            label={t('common.loading')}
                        />
                    ) : (
                        <EmptyState
                            variant="table"
                            title={t('common.no_matching_entries')}
                        />
                    )}
                </div>
            </DataTableSurface>

            <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                <div className="text-muted-foreground text-sm">
                    {t('view.notification.label.notifications_in_view', {
                        total: rowsCount
                    })}
                </div>
                <DataTablePagination
                    table={table}
                    pageIndex={pagination.pageIndex}
                    pageCount={table.getPageCount() || 1}
                    pageSize={pagination.pageSize}
                    pageSizes={pageSizes}
                    pageSizeLabel={t('table.pagination.rows_per_page')}
                    previousLabel={t('table.pagination.previous')}
                    nextLabel={t('table.pagination.next')}
                    onPageSizeChange={onPageSizeChange}
                />
            </div>
        </>
    );
}
