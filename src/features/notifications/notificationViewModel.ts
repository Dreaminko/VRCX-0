import type { NotificationRow } from '@/repositories/notificationPersistenceRepository';
import { convertFileUrlToImageUrl } from '@/services/entityMediaService';
import { hasGroupIdPrefix } from '@/shared/constants/vrchatIds';
import {
    isNotificationExpired,
    isUnseenNotification
} from '@/shared/utils/notificationSeen';

export const NOTIFICATION_TYPE_LABEL_PREFIX = 'view.notification.filters.';

export type NotificationActor =
    | { kind: 'user'; id: string; name: string; imageUrl: string }
    | { kind: 'group'; id: string; name: string; imageUrl: string }
    | { kind: 'system'; name: string };

export type NotificationViewModelContext = {
    location: string;
    worldName: string;
    groupName: string;
};

export type NotificationViewModelLink = {
    href: string;
    text: string;
    internal: boolean;
};

export type NotificationViewModel = {
    id: string;
    template: 'compact' | 'broadcast' | 'fallback';
    typeLabelKey: string;
    createdAt: string;
    actor: NotificationActor;
    headline: string;
    body: string;
    context: NotificationViewModelContext | null;
    media: string;
    link: NotificationViewModelLink | null;
    unseen: boolean;
    expired: boolean;
};

export type NotificationViewModelOptions = {
    unknownLabel?: string;
};

const BROADCAST_TYPES = new Set<string>([
    'group.announcement',
    'group.event.created'
]);
const INVITE_TYPES = new Set<string>([
    'invite',
    'requestInvite',
    'friendRequest',
    'ignoredFriendRequest',
    'inviteResponse',
    'requestInviteResponse'
]);
const PERSON_TYPES = new Set<string>(['boop', 'message']);
const KNOWN_TYPES = new Set<string>([
    'requestInvite',
    'invite',
    'requestInviteResponse',
    'inviteResponse',
    'friendRequest',
    'ignoredFriendRequest',
    'message',
    'boop',
    'event.announcement',
    'groupChange',
    'group.announcement',
    'group.event.created',
    'group.informative',
    'group.invite',
    'group.joinRequest',
    'group.transfer',
    'group.queueReady',
    'moderation.warning.group',
    'moderation.report.closed',
    'moderation.contentrestriction',
    'instance.closed',
    'economy.alert',
    'economy.received.gift',
    'badge.earned',
    'vrcplus.gift'
]);
const INTERNAL_LINK_SCHEMES = new Set<string>([
    'user',
    'group',
    'event',
    'world',
    'avatar'
]);

export function getNotificationLinkScheme(link: unknown): string {
    const value = String(link || '').trim();
    const separatorIndex = value.indexOf(':');
    if (separatorIndex <= 0) {
        return '';
    }
    return value.slice(0, separatorIndex).toLowerCase();
}

export function notificationLinkIsInternal(link: unknown): boolean {
    return INTERNAL_LINK_SCHEMES.has(getNotificationLinkScheme(link));
}

function text(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function firstText(...values: unknown[]): string {
    for (const value of values) {
        const normalized = text(value);
        if (normalized) {
            return normalized;
        }
    }
    return '';
}

function rawImageUrl(notification: NotificationRow): string {
    const imageUrl = firstText(
        notification.details?.imageUrl,
        notification.imageUrl,
        notification.senderUserIcon
    );
    return imageUrl.startsWith('default_') ? '' : imageUrl;
}

function thumbnailUrl(notification: NotificationRow): string {
    const imageUrl = rawImageUrl(notification);
    return imageUrl ? convertFileUrlToImageUrl(imageUrl, 64) : '';
}

function isGroupSender(notification: NotificationRow): boolean {
    const type = text(notification.type);
    return (
        hasGroupIdPrefix(text(notification.senderUserId)) ||
        type === 'groupChange' ||
        type.startsWith('group.') ||
        type.startsWith('moderation.')
    );
}

function groupActor(
    notification: NotificationRow
): Extract<NotificationActor, { kind: 'group' }> {
    const senderUserId = text(notification.senderUserId);
    return {
        kind: 'group',
        id: firstText(
            notification.data?.groupId,
            notification.data?.ownerId,
            notification.details?.groupId,
            hasGroupIdPrefix(senderUserId) ? senderUserId : ''
        ),
        name: firstText(
            notification.data?.groupName,
            notification.data?.ownerName,
            notification.details?.groupName,
            notification.groupName,
            notification.senderUsername
        ),
        imageUrl: thumbnailUrl(notification)
    };
}

function userActor(notification: NotificationRow): NotificationActor {
    return {
        kind: 'user',
        id: text(notification.senderUserId),
        name: firstText(
            notification.senderDisplayName,
            notification.details?.senderDisplayName,
            notification.data?.senderDisplayName,
            notification.senderUsername,
            notification.senderUserId
        ),
        imageUrl: thumbnailUrl(notification)
    };
}

function buildLink(
    notification: NotificationRow,
    actor: NotificationActor
): NotificationViewModelLink | null {
    const href = text(notification.link);
    if (!href) {
        return null;
    }
    if (getNotificationLinkScheme(href) === 'user') {
        const targetId = href.slice(href.indexOf(':') + 1).trim();
        const actorId = actor.kind === 'system' ? '' : actor.id;
        if (targetId && targetId === actorId) {
            return null;
        }
    }
    return {
        href,
        text: firstText(notification.linkText, href),
        internal: notificationLinkIsInternal(href)
    };
}

function buildContext(
    notification: NotificationRow,
    includeGroupName: boolean
): NotificationViewModelContext | null {
    const location = firstText(
        notification.details?.worldId,
        notification.data?.worldId,
        notification.location
    );
    if (!location) {
        return null;
    }
    return {
        location,
        worldName: firstText(
            notification.details?.worldName,
            notification.worldName,
            notification.data?.worldName
        ),
        groupName: includeGroupName
            ? firstText(
                  notification.details?.groupName,
                  notification.data?.groupName,
                  notification.groupName
              )
            : ''
    };
}

function inviteBody(notification: NotificationRow): string {
    return firstText(
        notification.details?.inviteMessage,
        notification.details?.requestMessage,
        notification.details?.responseMessage,
        notification.inviteMessage,
        notification.requestMessage,
        notification.responseMessage
    );
}

export function toNotificationViewModel(
    notification: NotificationRow,
    { unknownLabel = '' }: NotificationViewModelOptions = {}
): NotificationViewModel {
    const type = text(notification.type);
    const base = {
        id: text(notification.id),
        typeLabelKey: `${NOTIFICATION_TYPE_LABEL_PREFIX}${type || 'unknown'}`,
        createdAt: firstText(notification.createdAt, notification.created_at),
        unseen: isUnseenNotification(notification),
        expired: isNotificationExpired(notification)
    };

    if (!type || !KNOWN_TYPES.has(type)) {
        const actor: NotificationActor = { kind: 'system', name: unknownLabel };
        return {
            ...base,
            template: 'fallback',
            actor,
            headline: '',
            body: firstText(notification.message, notification.title),
            context: null,
            media: '',
            link: buildLink(notification, actor)
        };
    }

    if (BROADCAST_TYPES.has(type)) {
        const actor = groupActor(notification);
        const hasBanner = type === 'group.event.created';
        return {
            ...base,
            template: 'broadcast',
            actor: hasBanner ? { ...actor, imageUrl: '' } : actor,
            headline: firstText(
                notification.data?.announcementTitle,
                notification.data?.title
            ),
            body: text(notification.message),
            context: null,
            media: hasBanner ? rawImageUrl(notification) : '',
            link: null
        };
    }

    if (INVITE_TYPES.has(type)) {
        const actor = userActor(notification);
        return {
            ...base,
            template: 'compact',
            actor,
            headline: '',
            body: inviteBody(notification),
            context: buildContext(notification, false),
            media: '',
            link: buildLink(notification, actor)
        };
    }

    if (PERSON_TYPES.has(type)) {
        const actor = userActor(notification);
        return {
            ...base,
            template: 'compact',
            actor,
            headline: '',
            body: text(notification.message),
            context: buildContext(notification, true),
            media: '',
            link: buildLink(notification, actor)
        };
    }

    const actor: NotificationActor = isGroupSender(notification)
        ? groupActor(notification)
        : text(notification.senderUserId)
          ? userActor(notification)
          : { kind: 'system', name: firstText(notification.title) };
    return {
        ...base,
        template: 'compact',
        actor,
        headline: '',
        body: text(notification.message),
        context: buildContext(notification, true),
        media: '',
        link: buildLink(notification, actor)
    };
}
