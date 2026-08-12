import {
    commands,
    type NotificationActionOutcome,
    type NotificationTarget,
    type SocialFriendMutationOutcome
} from '@/platform/tauri/bindings';
import notificationPersistenceRepository from '@/repositories/notificationPersistenceRepository';

type NotificationRecord = Record<string, unknown> & {
    id?: unknown;
    version?: unknown;
    type?: unknown;
    senderUserId?: unknown;
    senderUsername?: unknown;
    expired?: unknown;
    link?: unknown;
};

interface NotificationActionInput {
    currentUserId?: unknown;
    notification?: NotificationRecord | null;
}

interface FriendRequestNotificationInput {
    notification?: NotificationRecord | null;
    targetUser?: NotificationRecord | null;
}

interface AcceptRequestInviteInput extends NotificationActionInput {
    instanceId?: unknown;
    worldId?: unknown;
}

interface InviteResponseInput extends NotificationActionInput {
    responseSlot?: unknown;
    imageData?: unknown;
    withUploadTimeout?: (promise: Promise<unknown>) => Promise<unknown>;
}

interface NotificationResponseInput extends NotificationActionInput {
    response?: NotificationRecord | null;
}

interface BoopReplyInput extends NotificationActionInput {
    emojiId?: unknown;
}

function normalizeText(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function requireNotification(
    notification: NotificationRecord | null | undefined
) {
    if (!notification) {
        throw new Error('Notification action requires a notification.');
    }
    return notification;
}

function toNotificationTarget(
    notification: NotificationRecord
): NotificationTarget {
    return {
        id: normalizeText(notification.id),
        version: Number(notification.version) || 0,
        type: normalizeText(notification.type),
        senderUserId: normalizeText(notification.senderUserId)
    };
}

function unwrapNotificationActionOutcome(
    outcome: NotificationActionOutcome
): NotificationActionOutcome {
    if (outcome.status === 'remoteFailed') {
        throw new Error(
            outcome.remoteError || 'VRChat notification request failed'
        );
    }
    if (outcome.status === 'remoteOkLocalFailed') {
        throw new Error(
            outcome.localError || 'Notification local update failed.'
        );
    }
    return outcome;
}

export async function findIncomingFriendRequestNotification({
    currentUserId,
    targetUserId
}: {
    currentUserId?: unknown;
    targetUserId?: unknown;
}) {
    const normalizedCurrentUserId = normalizeText(currentUserId);
    const normalizedTargetUserId = normalizeText(targetUserId);
    if (!normalizedCurrentUserId || !normalizedTargetUserId) {
        return null;
    }

    const rows = await notificationPersistenceRepository.queryNotifications({
        userId: normalizedCurrentUserId,
        filters: ['friendRequest']
    });
    return (
        rows.find(
            (row) =>
                row?.type === 'friendRequest' &&
                !row.expired &&
                normalizeText(row.senderUserId) === normalizedTargetUserId
        ) || null
    );
}

export async function expireNotificationLocally({
    currentUserId,
    notification
}: NotificationActionInput) {
    const target = requireNotification(notification);
    await notificationPersistenceRepository.expireNotification({
        userId: currentUserId,
        id: target.id
    });
}

export async function hideRemoteAndExpireNotification({
    currentUserId,
    notification
}: NotificationActionInput) {
    const target = requireNotification(notification);
    const outcome = await commands.appNotificationHideAndExpire({
        ownerUserId: normalizeText(currentUserId),
        target: toNotificationTarget(target)
    });
    unwrapNotificationActionOutcome(outcome);
}

export async function acceptFriendRequestNotification({
    notification,
    targetUser = null
}: FriendRequestNotificationInput): Promise<
    | { status: 'accepted'; outcome: SocialFriendMutationOutcome }
    | { status: 'not-found' }
> {
    const target = requireNotification(notification);
    const targetUserId = normalizeText(target.senderUserId);
    const targetDisplayName =
        normalizeText(targetUser?.displayName) ||
        normalizeText(target.senderUsername);

    const result = await commands.appSocialFriendRequestNotificationAccept({
        notificationId: normalizeText(target.id),
        targetUserId,
        targetDisplayName
    });
    if (result.status === 'notFound') {
        return { status: 'not-found' as const };
    }
    if (!result.outcome) {
        throw new Error('Friend request accept result is incomplete.');
    }
    return { status: 'accepted' as const, outcome: result.outcome };
}

export async function acceptRequestInviteNotification({
    currentUserId,
    notification,
    instanceId,
    worldId
}: AcceptRequestInviteInput) {
    const target = requireNotification(notification);
    const notificationTarget = toNotificationTarget(target);
    const normalizedInstanceId = normalizeText(instanceId);
    const outcome = await commands.appNotificationRequestInviteAccept({
        ownerUserId: normalizeText(currentUserId),
        target: notificationTarget,
        instanceId: normalizedInstanceId,
        worldId: normalizeText(worldId)
    });
    unwrapNotificationActionOutcome(outcome);
}

export async function sendInviteResponseNotification({
    currentUserId,
    notification,
    responseSlot,
    imageData,
    withUploadTimeout
}: InviteResponseInput) {
    const target = requireNotification(notification);
    const normalizedResponseSlot = Number.parseInt(
        String(responseSlot ?? ''),
        10
    );
    if (!Number.isFinite(normalizedResponseSlot)) {
        throw new Error('Response slot must be a number.');
    }

    const invoke = () =>
        commands.appNotificationInviteResponseSend({
            ownerUserId: normalizeText(currentUserId),
            target: toNotificationTarget(target),
            responseSlot: normalizedResponseSlot,
            imageData: imageData ? normalizeText(imageData) : ''
        });
    const outcome =
        imageData && withUploadTimeout
            ? await withUploadTimeout(invoke())
            : await invoke();
    unwrapNotificationActionOutcome(outcome as NotificationActionOutcome);
    return { sentPhoto: Boolean(imageData) };
}

export async function dismissBoopNotifications({
    currentUserId,
    senderUserId
}: {
    currentUserId?: unknown;
    senderUserId?: unknown;
}) {
    const normalizedSenderUserId = normalizeText(senderUserId);
    if (!currentUserId || !normalizedSenderUserId) {
        return;
    }
    await commands.appNotificationBoopDismiss({
        ownerUserId: normalizeText(currentUserId),
        senderUserId: normalizedSenderUserId
    });
}

export async function sendBoopReplyNotification({
    currentUserId,
    notification,
    emojiId = ''
}: BoopReplyInput) {
    const target = requireNotification(notification);
    const senderUserId = normalizeText(target.senderUserId);
    if (!senderUserId) {
        throw new Error('Cannot send boop: no sender user id is available.');
    }
    const outcome = await commands.appNotificationBoopReply({
        ownerUserId: normalizeText(currentUserId),
        target: toNotificationTarget(target),
        emojiId: normalizeText(emojiId)
    });
    unwrapNotificationActionOutcome(outcome);
}

export async function sendNotificationButtonResponse({
    currentUserId,
    notification,
    response
}: NotificationResponseInput) {
    const target = requireNotification(notification);
    const outcome = await commands.appNotificationRespondAndExpire({
        ownerUserId: normalizeText(currentUserId),
        target: toNotificationTarget(target),
        responseType: normalizeText(response?.type),
        responseData: response?.data || ''
    });
    unwrapNotificationActionOutcome(outcome);
}
