import { commands } from '@/platform/tauri/bindings';

import { unwrapVrchatResponse } from './vrchatRequest';

type FriendRecord = Record<string, unknown> & { id: string };

interface FriendEndpointInput {
    userId?: unknown;
    isFriend?: boolean | null;
}

function unwrapVrchatFriendResponse<TJson = unknown>(
    response: { status: number; data: unknown },
    path: string
) {
    return unwrapVrchatResponse<TJson>(response, path, {
        fallbackMessage: 'VRChat friend request failed'
    });
}

async function getUser({ userId, isFriend = null }: FriendEndpointInput) {
    const normalizedUserId =
        typeof userId === 'string'
            ? userId.trim()
            : String(userId ?? '').trim();
    if (!normalizedUserId) {
        throw new Error('VrchatFriendRepository.getUser requires a user id.');
    }

    const response = await commands.appVrchatUserGet({
        userId: normalizedUserId,
        isFriend
    });
    return unwrapVrchatFriendResponse<FriendRecord>(
        response,
        `users/${encodeURIComponent(normalizedUserId)}`
    );
}

const vrchatFriendRepository = Object.freeze({
    getUser
});

export { getUser };
export default vrchatFriendRepository;
