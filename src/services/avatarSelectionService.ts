import avatarProfileRepository from '@/repositories/avatarProfileRepository';
import { mergeCurrentUserResponseSnapshot } from '@/shared/utils/currentUserSnapshot';
import {
    useRuntimeStore,
    type CurrentUserSnapshotState
} from '@/state/runtimeStore';

import { buildAvatarWearSnapshotUpdate } from './avatarWearTimeService';
import { recordCurrentUserSnapshot } from './domainIngestionService';

type CurrentUserResponse = CurrentUserSnapshotState & {
    id: string;
};

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

type AvatarSelectionKind = 'avatar' | 'fallback';

const RESPONSE_AUTHORITY_FIELDS: Record<
    AvatarSelectionKind,
    ReadonlySet<string>
> = {
    avatar: new Set([
        'currentAvatar',
        'currentAvatarImageUrl',
        'currentAvatarName',
        'currentAvatarTags',
        'currentAvatarThumbnailImageUrl'
    ]),
    fallback: new Set(['fallbackAvatar'])
};

let selectionSequence = 0;
const latestAppliedSelectionSequence: Record<AvatarSelectionKind, number> = {
    avatar: 0,
    fallback: 0
};

function isCurrentUserResponse(value: unknown): value is CurrentUserResponse {
    return Boolean(
        value &&
        typeof value === 'object' &&
        !Array.isArray(value) &&
        'id' in value &&
        typeof value.id === 'string' &&
        value.id.trim()
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

function getCurrentUserDisplayName(user: CurrentUserResponse): string {
    return user.displayName || user.username || user.id;
}

async function selectAvatarWithCurrentUserResponse(
    kind: AvatarSelectionKind,
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
    const baseSnapshot = runtimeStore.auth.currentUserSnapshot;
    const sequence = ++selectionSequence;
    const response = await request();
    if (!isCurrentAuthTarget(target)) {
        return { applied: false };
    }
    if (!isCurrentUserResponse(response.json)) {
        throw new Error(
            'VRChat avatar selection returned an invalid current user.'
        );
    }
    const responseUserId = response.json.id.trim();
    if (
        responseUserId !== target.currentUserId ||
        sequence < latestAppliedSelectionSequence[kind]
    ) {
        return { applied: false };
    }

    const currentState = useRuntimeStore.getState();
    const responseUser =
        responseUserId === response.json.id
            ? response.json
            : { ...response.json, id: responseUserId };
    const mergedUser = mergeCurrentUserResponseSnapshot({
        responseUser,
        baseSnapshot,
        currentSnapshot: currentState.auth.currentUserSnapshot,
        overlayPatch: null,
        responseAuthorityFields: RESPONSE_AUTHORITY_FIELDS[kind]
    });
    const { snapshot } = buildAvatarWearSnapshotUpdate({
        previousSnapshot: currentState.auth.currentUserSnapshot,
        nextSnapshot: mergedUser,
        isGameRunning: currentState.gameState.isGameRunning
    });
    const nextUser = isCurrentUserResponse(snapshot) ? snapshot : responseUser;

    latestAppliedSelectionSequence[kind] = sequence;
    currentState.setAuthBootstrap({
        currentUserId: nextUser.id,
        currentUserDisplayName: getCurrentUserDisplayName(nextUser),
        currentUserSnapshot: nextUser
    });
    recordCurrentUserSnapshot(nextUser, {
        endpoint: target.currentUserEndpoint
    });
    return { applied: true };
}

export function selectAvatar(avatarId: string) {
    return selectAvatarWithCurrentUserResponse('avatar', () =>
        avatarProfileRepository.selectAvatar({ avatarId })
    );
}

export function selectFallbackAvatar(avatarId: string) {
    return selectAvatarWithCurrentUserResponse('fallback', () =>
        avatarProfileRepository.selectFallbackAvatar({ avatarId })
    );
}
