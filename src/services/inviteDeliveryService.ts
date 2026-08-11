import { commands } from '@/platform/tauri/bindings';
import notificationPersistenceRepository from '@/repositories/notificationPersistenceRepository';
import type { QueryParams } from '@/repositories/vrchatRequest';

interface SendInviteToLocationInput {
    receiverUserId?: unknown;
    instanceId?: unknown;
    worldId?: unknown;
    worldName?: unknown;
    messageSlot?: unknown;
    imageData?: unknown;
    rsvp?: unknown;
}

interface SendInvitesToLocationInput {
    receiverUserIds?: unknown[];
    location?: unknown;
    shortName?: unknown;
    worldName?: unknown;
}

interface SendRequestInviteToUserInput {
    receiverUserId?: unknown;
    platform?: string;
    requestSlot?: unknown;
    imageData?: unknown;
}

interface SendBoopToUserInput {
    userId?: unknown;
    emojiId?: unknown;
}

function normalizeText(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

export async function sendInvitesToLocation({
    receiverUserIds = [],
    location,
    shortName,
    worldName
}: SendInvitesToLocationInput = {}) {
    return commands.appInstanceInviteBatch({
        receiverUserIds: receiverUserIds.map(normalizeText).filter(Boolean),
        location: normalizeText(location),
        shortName: normalizeText(shortName),
        worldName: normalizeText(worldName)
    });
}

export async function sendInviteToLocation({
    receiverUserId,
    instanceId,
    worldId,
    worldName,
    messageSlot = null,
    imageData = '',
    rsvp
}: SendInviteToLocationInput = {}) {
    const normalizedReceiverUserId = normalizeText(receiverUserId);
    const normalizedInstanceId = normalizeText(instanceId);
    const normalizedWorldId = normalizeText(worldId);
    if (
        !normalizedReceiverUserId ||
        !normalizedInstanceId ||
        !normalizedWorldId
    ) {
        return null;
    }

    const normalizedMessageSlot = Number.parseInt(
        String(messageSlot ?? ''),
        10
    );
    const outcome = await commands.appNotificationInstanceInviteSend({
        receiverUserId: normalizedReceiverUserId,
        instanceId: normalizedInstanceId,
        worldId: normalizedWorldId,
        worldName: normalizeText(worldName),
        messageSlot: Number.isFinite(normalizedMessageSlot)
            ? normalizedMessageSlot
            : null,
        imageData: normalizeText(imageData),
        rsvp: typeof rsvp === 'boolean' ? rsvp : null
    });
    if (outcome.status === 'remoteFailed') {
        throw new Error(
            outcome.remoteError || 'VRChat notification request failed'
        );
    }
    return outcome;
}

export async function sendRequestInviteToUser({
    receiverUserId,
    platform = 'standalonewindows',
    requestSlot = null,
    imageData = ''
}: SendRequestInviteToUserInput = {}) {
    const normalizedReceiverUserId = normalizeText(receiverUserId);
    if (!normalizedReceiverUserId) {
        return null;
    }

    const params: QueryParams = { platform };
    const normalizedRequestSlot = Number.parseInt(
        String(requestSlot ?? ''),
        10
    );
    if (Number.isFinite(normalizedRequestSlot)) {
        params.requestSlot = normalizedRequestSlot;
    }

    const normalizedImageData = normalizeText(imageData);
    if (normalizedImageData) {
        return notificationPersistenceRepository.sendRequestInvitePhoto({
            receiverUserId: normalizedReceiverUserId,
            params,
            imageData: normalizedImageData
        });
    }

    return notificationPersistenceRepository.sendRequestInvite({
        receiverUserId: normalizedReceiverUserId,
        params
    });
}

export async function sendBoopToUser({
    userId,
    emojiId = ''
}: SendBoopToUserInput = {}) {
    const normalizedUserId = normalizeText(userId);
    if (!normalizedUserId) {
        return null;
    }

    return notificationPersistenceRepository.sendBoop({
        userId: normalizedUserId,
        emojiId
    });
}
