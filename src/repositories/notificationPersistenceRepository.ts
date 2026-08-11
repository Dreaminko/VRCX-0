import {
    commands,
    type HttpApiExecuteResponse,
    type NotificationListItemOutput,
    type NotificationListQueryInput,
    type VrchatBoopInput,
    type VrchatNotificationPhotoSendInput,
    type VrchatNotificationSendInput
} from '@/platform/tauri/bindings';

import configRepository from './configRepository';
import { type QueryParams, unwrapVrchatResponse } from './vrchatRequest';

export type NotificationDetails = Record<string, unknown> & {
    displayLocation?: string;
    emojiId?: unknown;
    groupId?: string;
    groupName?: string;
    imageUrl?: string;
    inviteMessage?: string;
    requestMessage?: string;
    responseMessage?: string;
    senderDisplayName?: string;
    worldId?: string;
    worldName?: string;
};
export type NotificationData = Record<string, unknown> & {
    announcementTitle?: string;
    groupId?: string;
    groupName?: string;
    senderDisplayName?: string;
};
export type NotificationResponse = Record<string, unknown> & {
    data?: unknown;
    icon?: string;
    text?: string;
    textKey?: string;
    type?: string;
};
export type NotificationListRow = Omit<
    NotificationListItemOutput,
    'details' | 'data' | 'responses'
> & {
    details: NotificationDetails;
    data: NotificationData;
    responses: NotificationResponse[];
};
export type NotificationRow = Omit<
    Partial<NotificationListRow>,
    'createdAt' | 'created_at' | 'updatedAt' | 'expiresAt'
> &
    Record<string, unknown> & {
        createdAt?: string | number | null;
        created_at?: string | number | null;
        updatedAt?: string | number | null;
        expiresAt?: string | null;
        displayLocation?: string;
        groupName?: string;
        location?: string;
        senderDisplayName?: string;
        senderUserIcon?: string;
        worldName?: string;
    };

type NotificationRecord = NotificationRow;

interface NotificationUserOptions {
    userId?: unknown;
}

interface NotificationActionOptions {
    imageData?: unknown;
    receiverUserId?: unknown;
    userId?: unknown;
    emojiId?: unknown;
    params?: QueryParams;
}

export const NOTIFICATION_TYPES = Object.freeze([
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

function normalizeUserId(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeNotificationFilters(filters: unknown): string[] {
    return Array.isArray(filters)
        ? filters.map((value) => String(value || '').trim()).filter(Boolean)
        : [];
}

function normalizeNotificationLimit(value: unknown, fallback: number): number {
    const limit = Number.parseInt(String(value ?? ''), 10);
    return Number.isFinite(limit) && limit > 0 ? limit : fallback;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function normalizeNotificationObject(value: unknown): Record<string, unknown> {
    return isRecord(value) ? value : {};
}

function normalizeNotificationResponses(
    value: unknown
): NotificationResponse[] {
    return Array.isArray(value)
        ? value.filter(isRecord).map((response) => ({ ...response }))
        : [];
}

function normalizeNotificationListRow(
    row: NotificationListItemOutput
): NotificationListRow {
    return {
        ...row,
        details: normalizeNotificationObject(row.details),
        data: normalizeNotificationObject(row.data),
        responses: normalizeNotificationResponses(row.responses)
    };
}

function unwrapVrchatNotificationResponse<TJson = NotificationRecord>(
    response: HttpApiExecuteResponse,
    path: string
) {
    return unwrapVrchatResponse<TJson>(response, path, {
        fallbackMessage: 'VRChat notification request failed'
    });
}

async function queryNotifications({
    userId,
    search = '',
    filters = []
}: NotificationUserOptions & {
    search?: string;
    filters?: unknown[];
} = {}): Promise<NotificationListRow[]> {
    const normalizedUserId = normalizeUserId(userId);
    if (!normalizedUserId) {
        return [];
    }

    const normalizedSearch = String(search || '').trim();
    const normalizedFilters = normalizeNotificationFilters(filters);
    const [maxTableSize, searchLimit] = await Promise.all([
        configRepository.getInt('maxTableSize_v2', 500),
        configRepository.getInt('searchLimit', 50000)
    ]);
    const isSearchOrFiltered =
        Boolean(normalizedSearch) || normalizedFilters.length > 0;
    const limit = isSearchOrFiltered
        ? normalizeNotificationLimit(searchLimit, 50000)
        : normalizeNotificationLimit(maxTableSize, 500);
    const perTableLimit = isSearchOrFiltered ? limit : limit * 2;
    const isDefaultList = !normalizedSearch && normalizedFilters.length === 0;
    const query = {
        userId: normalizedUserId,
        search: normalizedSearch,
        filters: normalizedFilters,
        perTableLimit,
        limit,
        includeUnseen: isDefaultList
    } satisfies NotificationListQueryInput;
    const rows = await commands.appNotificationListQuery(query);
    return rows.map(normalizeNotificationListRow);
}

async function addNotificationToDatabase({
    userId,
    notification
}: NotificationUserOptions & { notification?: NotificationRecord } = {}) {
    const normalizedUserId = normalizeUserId(userId);
    if (!normalizedUserId) {
        return;
    }

    const notificationDetails = isRecord(notification?.details)
        ? notification.details
        : {};
    const entry: NotificationRecord & { details: Record<string, unknown> } = {
        id: '',
        created_at: '',
        type: '',
        senderUserId: '',
        senderUsername: '',
        receiverUserId: '',
        message: '',
        ...(notification || {}),
        details: {
            worldId: '',
            worldName: '',
            imageUrl: '',
            inviteMessage: '',
            requestMessage: '',
            responseMessage: '',
            ...notificationDetails
        }
    };
    if (entry.imageUrl && !entry.details.imageUrl) {
        entry.details.imageUrl = entry.imageUrl;
    }
    if (!entry.created_at || !entry.type || !entry.id) {
        throw new Error('Notification is missing required field');
    }

    await commands.appNotificationAddV1(normalizedUserId, entry);
}

async function addNotificationV2ToDatabase({
    userId,
    notification
}: NotificationUserOptions & { notification?: NotificationRecord } = {}) {
    const normalizedUserId = normalizeUserId(userId);
    if (!normalizedUserId || !notification?.id) {
        return;
    }

    await commands.appNotificationAddV2(normalizedUserId, notification);
}

async function expireNotificationV2({
    userId,
    id
}: NotificationUserOptions & { id?: unknown } = {}) {
    const normalizedUserId = normalizeUserId(userId);
    const normalizedId = normalizeUserId(id);
    if (!normalizedUserId || !normalizedId) {
        return;
    }

    await commands.appNotificationV2Expire(normalizedUserId, normalizedId);
}

async function seenNotificationV2({
    userId,
    id
}: NotificationUserOptions & { id?: unknown } = {}) {
    const normalizedUserId = normalizeUserId(userId);
    const normalizedId = normalizeUserId(id);
    if (!normalizedUserId || !normalizedId) {
        return;
    }

    await commands.appNotificationV2MarkSeen(normalizedUserId, normalizedId);
}

async function updateNotificationExpired({
    userId,
    notification
}: NotificationUserOptions & { notification?: NotificationRecord } = {}) {
    const normalizedUserId = normalizeUserId(userId);
    const normalizedId = normalizeUserId(notification?.id);
    if (!normalizedUserId || !normalizedId) {
        return;
    }

    await commands.appNotificationUpdateExpired(
        normalizedUserId,
        normalizedId,
        Boolean(notification?.$isExpired)
    );
}

async function deleteNotification({
    userId,
    id
}: NotificationUserOptions & { id?: unknown; version?: unknown }) {
    const normalizedUserId = normalizeUserId(userId);
    const normalizedId =
        typeof id === 'string' ? id.trim() : String(id ?? '').trim();
    if (!normalizedUserId || !normalizedId) {
        return;
    }

    await commands.appNotificationDelete(normalizedUserId, normalizedId);
}

async function expireNotification({
    userId,
    id
}: NotificationUserOptions & { id?: unknown }) {
    const normalizedUserId = normalizeUserId(userId);
    const normalizedId =
        typeof id === 'string' ? id.trim() : String(id ?? '').trim();
    if (!normalizedUserId || !normalizedId) {
        return;
    }

    await commands.appNotificationExpire(normalizedUserId, normalizedId);
}

async function sendRequestInvite({
    receiverUserId,
    params = {}
}: NotificationActionOptions = {}) {
    const normalizedReceiverUserId =
        typeof receiverUserId === 'string'
            ? receiverUserId.trim()
            : String(receiverUserId ?? '').trim();
    if (!normalizedReceiverUserId) {
        return null;
    }

    const input = {
        receiverUserId: normalizedReceiverUserId,
        params
    } satisfies VrchatNotificationSendInput;
    const response = await commands.appVrchatRequestInviteSend(input);
    return unwrapVrchatNotificationResponse(
        response,
        `requestInvite/${encodeURIComponent(normalizedReceiverUserId)}`
    );
}

async function sendRequestInvitePhoto({
    receiverUserId,
    params = {},
    imageData
}: NotificationActionOptions = {}) {
    const normalizedReceiverUserId =
        typeof receiverUserId === 'string'
            ? receiverUserId.trim()
            : String(receiverUserId ?? '').trim();
    const normalizedImageData =
        typeof imageData === 'string'
            ? imageData.trim()
            : String(imageData ?? '').trim();
    if (!normalizedReceiverUserId || !normalizedImageData) {
        return null;
    }

    const input = {
        receiverUserId: normalizedReceiverUserId,
        params,
        imageData: normalizedImageData
    } satisfies VrchatNotificationPhotoSendInput;
    const response = await commands.appVrchatRequestInvitePhotoSend(input);
    return unwrapVrchatNotificationResponse(
        response,
        `requestInvite/${encodeURIComponent(normalizedReceiverUserId)}/photo`
    );
}

async function sendBoop({
    userId,
    emojiId = ''
}: NotificationActionOptions = {}) {
    const normalizedUserId =
        typeof userId === 'string'
            ? userId.trim()
            : String(userId ?? '').trim();
    if (!normalizedUserId) {
        return null;
    }

    const normalizedEmojiId =
        typeof emojiId === 'string'
            ? emojiId.trim()
            : String(emojiId ?? '').trim();
    const input = {
        userId: normalizedUserId,
        emojiId: normalizedEmojiId
    } satisfies VrchatBoopInput;
    const response = await commands.appVrchatBoopSend(input);
    return unwrapVrchatNotificationResponse(
        response,
        `users/${encodeURIComponent(normalizedUserId)}/boop`
    );
}

const notificationPersistenceRepository = Object.freeze({
    addNotificationToDatabase,
    addNotificationV2ToDatabase,
    expireNotificationV2,
    queryNotifications,
    deleteNotification,
    expireNotification,
    sendRequestInvite,
    sendRequestInvitePhoto,
    sendBoop,
    seenNotificationV2,
    updateNotificationExpired
});

export {
    addNotificationToDatabase,
    addNotificationV2ToDatabase,
    expireNotificationV2,
    queryNotifications,
    deleteNotification,
    expireNotification,
    sendRequestInvite,
    sendRequestInvitePhoto,
    sendBoop,
    seenNotificationV2,
    updateNotificationExpired
};
export default notificationPersistenceRepository;
