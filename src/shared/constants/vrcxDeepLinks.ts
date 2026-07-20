import { isAvatarId, isWorldId } from './vrchatIds';

export const VRCX_OPEN_RELAY_ORIGIN = 'https://open.vrcx-0.dev';

function entityRelayLink(entity: 'avatar' | 'world', entityId: string): string {
    return `${VRCX_OPEN_RELAY_ORIGIN}/${entity}/${entityId.trim()}`;
}

export function vrcxWorldDeepLink(worldId: unknown): string {
    return isWorldId(worldId) ? entityRelayLink('world', String(worldId)) : '';
}

export function vrcxAvatarDeepLink(avatarId: unknown): string {
    return isAvatarId(avatarId)
        ? entityRelayLink('avatar', String(avatarId))
        : '';
}
