import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { getNotificationLifecycleBucket } from '@/shared/utils/notificationLifecycle';
import {
    isNotificationExpired,
    isUnseenNotification
} from '@/shared/utils/notificationSeen';
import { useVrcNotificationStore } from '@/state/vrcNotificationStore';

import type {
    NotificationLoadStatus,
    NotificationRow
} from './notificationPageTypes';
import { filterNotificationRows } from './notificationRows';
import type { NotificationQuickFilter } from './useNotificationFilters';

function matchesQuickFilter(
    notification: NotificationRow,
    quickFilter: NotificationQuickFilter
): boolean {
    if (quickFilter === 'unread') {
        return isUnseenNotification(notification);
    }
    if (quickFilter === 'action') {
        return (
            getNotificationLifecycleBucket(notification.type) === 'action' &&
            !isNotificationExpired(notification)
        );
    }
    return true;
}

export function useNotificationRows({
    activeTypes,
    currentUserId,
    deferredSearchQuery,
    filtersReady,
    quickFilter
}: {
    activeTypes: string[];
    currentUserId?: string;
    deferredSearchQuery: string;
    filtersReady: boolean;
    quickFilter: NotificationQuickFilter;
}) {
    const { t } = useTranslation();
    const notificationRows = useVrcNotificationStore((state) => state.rows);
    const notificationLoadStatus = useVrcNotificationStore(
        (state) => state.loadStatus
    );
    const notificationDetail = useVrcNotificationStore((state) => state.detail);
    const loadNotificationsForCurrentUser = useVrcNotificationStore(
        (state) => state.loadForCurrentUser
    );
    const [rows, setRows] = useState<NotificationRow[]>([]);
    const [loadStatus, setLoadStatus] =
        useState<NotificationLoadStatus>('idle');
    const [detail, setDetail] = useState('');
    const [reloadToken, setReloadToken] = useState(0);

    const reload = useCallback(() => {
        setReloadToken((value) => value + 1);
    }, []);

    useEffect(() => {
        let active = true;
        if (!filtersReady) {
            return () => {
                active = false;
            };
        }
        if (!currentUserId) {
            setRows([]);
            setLoadStatus('idle');
            setDetail('No current user session is available.');
            return () => {
                active = false;
            };
        }
        loadNotificationsForCurrentUser().catch((error: unknown) => {
            if (!active) {
                return;
            }
            toast.error(
                error instanceof Error
                    ? error.message
                    : t('view.notifications.toast.failed_to_load_notifications')
            );
        });
        return () => {
            active = false;
        };
    }, [
        currentUserId,
        filtersReady,
        loadNotificationsForCurrentUser,
        reloadToken,
        t
    ]);

    useEffect(() => {
        if (!filtersReady || !currentUserId) {
            return;
        }
        const nextRows = filterNotificationRows(
            notificationRows,
            activeTypes,
            deferredSearchQuery
        ).filter((notification) =>
            matchesQuickFilter(notification, quickFilter)
        );
        setRows(nextRows);
        setLoadStatus(notificationLoadStatus);
        setDetail(notificationDetail || '');
    }, [
        activeTypes,
        currentUserId,
        deferredSearchQuery,
        filtersReady,
        notificationDetail,
        notificationLoadStatus,
        notificationRows,
        quickFilter
    ]);

    return {
        detail,
        loadStatus,
        reload,
        rows
    };
}
