import { DAY_MS } from '@/shared/constants/time';

import { getNotificationLifecycleBucket } from './notificationLifecycle';

export const RECENT_WINDOW_MS = DAY_MS;
export const ACTION_REQUIRED_V1_TYPES = new Set<string>(['friendRequest']);

export type NotificationSeenLike = {
    $isExpired?: unknown;
    createdAt?: string | number | null;
    created_at?: string | number | null;
    expired?: unknown;
    expiresAt?: string | null;
    seen?: unknown;
    type?: unknown;
    version?: unknown;
};

export function isNotificationExpired(
    notification?: NotificationSeenLike | null
): boolean {
    if (notification?.$isExpired !== undefined) {
        return Boolean(notification.$isExpired);
    }
    if (notification?.expired !== undefined) {
        return Boolean(notification.expired);
    }
    if (!notification?.expiresAt) {
        return false;
    }
    const expiresAt = Date.parse(notification.expiresAt);
    return Number.isFinite(expiresAt) && expiresAt <= Date.now();
}

export function isUnseenNotification(
    notification?: NotificationSeenLike | null
): boolean {
    if (!notification) {
        return false;
    }
    const version = Number(notification.version ?? 1);
    const type = String(notification.type || '');
    const isActionRequiredV1 =
        version !== 2 && ACTION_REQUIRED_V1_TYPES.has(type);
    return (
        (version === 2 || isActionRequiredV1) &&
        notification.seen === false &&
        !isNotificationExpired(notification)
    );
}

export function shouldBulkMarkSeen(
    notification?: NotificationSeenLike | null
): boolean {
    const version = Number(notification?.version ?? 1);
    const type = String(notification?.type || '');
    return !(version !== 2 && ACTION_REQUIRED_V1_TYPES.has(type));
}

export function shouldMarkSeenRemotely(
    notification?: NotificationSeenLike | null
): boolean {
    return (
        shouldBulkMarkSeen(notification) &&
        getNotificationLifecycleBucket(notification?.type) !== 'system'
    );
}
