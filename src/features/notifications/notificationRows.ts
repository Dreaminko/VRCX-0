import type { NotificationRow } from '@/repositories/notificationPersistenceRepository';
import { parseLocation } from '@/shared/utils/location';
export { resolveCurrentInviteLocation } from '@/shared/utils/invite';

type CachedInstanceLike = Record<string, unknown> & {
    closedAt?: unknown;
    instance?: CachedInstanceLike;
    instanceId?: string;
    location?: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

export function matchesNotificationSearch(
    notification: NotificationRow,
    search: unknown
): boolean {
    const query = String(search || '')
        .trim()
        .toLowerCase();
    if (!query) {
        return true;
    }

    return [
        notification.type,
        notification.senderDisplayName,
        notification.senderUsername,
        notification.senderUserId,
        notification.title,
        notification.message,
        notification.linkText,
        notification.link,
        notification.details?.worldName,
        notification.details?.worldId,
        notification.details?.inviteMessage,
        notification.details?.requestMessage,
        notification.details?.responseMessage,
        notification.data?.groupName
    ].some((value) =>
        String(value || '')
            .toLowerCase()
            .includes(query)
    );
}

export function filterNotificationRows(
    rows: readonly NotificationRow[] | null | undefined,
    filters: readonly string[] | null | undefined,
    search: unknown
): NotificationRow[] {
    const activeFilters = Array.isArray(filters) ? filters : [];
    const inputRows = Array.isArray(rows) ? rows : [];
    return inputRows.filter((notification) => {
        if (
            activeFilters.length &&
            !activeFilters.includes(String(notification.type || ''))
        ) {
            return false;
        }
        return matchesNotificationSearch(notification, search);
    });
}

export function normalizeWorldTarget(value: unknown): string {
    const text =
        typeof value === 'string' ? value.trim() : String(value ?? '').trim();
    return parseLocation(text).worldId || text.split(':')[0] || text;
}

export function getCachedInstanceLocation(instance: unknown) {
    if (!isRecord(instance)) {
        return '';
    }
    const nestedInstance = isRecord(instance.instance)
        ? instance.instance
        : null;
    return String(
        instance.location ||
            nestedInstance?.location ||
            instance.instanceId ||
            ''
    ).trim();
}

export function buildCachedInstanceMap(
    instances: readonly unknown[] | null | undefined
) {
    const map = new Map<string, CachedInstanceLike>();
    for (const instance of Array.isArray(instances) ? instances : []) {
        if (!isRecord(instance)) {
            continue;
        }
        const location = getCachedInstanceLocation(instance);
        if (location) {
            const nestedInstance = isRecord(instance.instance)
                ? instance.instance
                : null;
            map.set(location, nestedInstance || instance);
        }
    }
    return map;
}
