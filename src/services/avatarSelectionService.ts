import avatarProfileRepository from '@/repositories/avatarProfileRepository';
import { useRuntimeStore } from '@/state/runtimeStore';

type AuthTarget = {
    currentUserId: string;
    currentUserEndpoint: string;
    currentUserWebsocket: string;
};

type AvatarSelectionResponse = Awaited<
    ReturnType<typeof avatarProfileRepository.selectAvatar>
>;

type AvatarSelectionResult = {
    applied: boolean;
};

function isCurrentUserResponse(value: unknown): boolean {
    return Boolean(
        value &&
        typeof value === 'object' &&
        !Array.isArray(value) &&
        'id' in value &&
        typeof (value as { id: unknown }).id === 'string' &&
        (value as { id: string }).id.trim()
    );
}

function isCurrentAuthTarget(target: AuthTarget): boolean {
    const auth = useRuntimeStore.getState().auth;
    return (
        auth.currentUserId?.trim() === target.currentUserId &&
        auth.currentUserEndpoint === target.currentUserEndpoint &&
        auth.currentUserWebsocket === target.currentUserWebsocket
    );
}

async function selectAvatarWithCurrentUserResponse(
    request: () => Promise<AvatarSelectionResponse>
): Promise<AvatarSelectionResult> {
    const runtimeStore = useRuntimeStore.getState();
    const currentUserId = runtimeStore.auth.currentUserId?.trim() || '';
    if (!currentUserId) {
        throw new Error('VRChat avatar selection requires a current user.');
    }
    const target: AuthTarget = {
        currentUserId,
        currentUserEndpoint: runtimeStore.auth.currentUserEndpoint,
        currentUserWebsocket: runtimeStore.auth.currentUserWebsocket
    };
    const response = await request();
    if (!isCurrentAuthTarget(target)) {
        return { applied: false };
    }
    if (!isCurrentUserResponse(response.json)) {
        throw new Error(
            'VRChat avatar selection returned an invalid current user.'
        );
    }
    return { applied: response.applied };
}

export function selectAvatar(avatarId: string) {
    return selectAvatarWithCurrentUserResponse(() =>
        avatarProfileRepository.selectAvatar({ avatarId })
    );
}

export function selectFallbackAvatar(avatarId: string) {
    return selectAvatarWithCurrentUserResponse(() =>
        avatarProfileRepository.selectFallbackAvatar({ avatarId })
    );
}
