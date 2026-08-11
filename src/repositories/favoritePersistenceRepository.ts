import {
    commands,
    type LocalFavoriteGroupInput as IpcLocalFavoriteGroupInput,
    type LocalFavoriteGroupRenameInput as IpcLocalFavoriteGroupRenameInput,
    type LocalFavoriteInput as IpcLocalFavoriteInput,
    type FavoriteRow
} from '@/platform/tauri/bindings';

import configRepository from './configRepository';

export type LocalFavoriteKind = 'friend' | 'avatar' | 'world';

export interface WorldFavoriteRow {
    created_at: string;
    worldId: string;
    groupName: string;
}

export interface AvatarFavoriteRow {
    created_at: string;
    avatarId: string;
    groupName: string;
}

export interface FriendFavoriteRow {
    created_at: string;
    userId: string;
    groupName: string;
}

interface LocalFavoriteInput {
    kind?: LocalFavoriteKind;
    entityId?: unknown;
    groupName?: unknown;
}

interface LocalFavoriteGroupInput {
    kind?: LocalFavoriteKind;
    groupName?: unknown;
}

interface RenameLocalFavoriteGroupInput extends LocalFavoriteGroupInput {
    newGroupName?: unknown;
}

const LOCAL_FAVORITE_GROUP_CONFIG_KEYS = Object.freeze({
    friend: 'localFavoriteFriendGroups',
    avatar: 'localFavoriteAvatarGroups',
    world: 'localFavoriteWorldGroups'
} satisfies Record<LocalFavoriteKind, string>);

function isLocalFavoriteKind(kind: unknown): kind is LocalFavoriteKind {
    return kind === 'friend' || kind === 'avatar' || kind === 'world';
}

function getLocalFavoriteGroupConfigKey(kind: unknown): string | undefined {
    return isLocalFavoriteKind(kind)
        ? LOCAL_FAVORITE_GROUP_CONFIG_KEYS[kind]
        : undefined;
}

function applyLocalFavoriteGroupWrite(write: {
    configKey: string;
    groupNames: string[];
}): void {
    configRepository.applyServerEntry(
        write.configKey,
        JSON.stringify(write.groupNames)
    );
}

function normalizeWorldFavoriteRow(row: FavoriteRow): WorldFavoriteRow {
    return {
        created_at: normalizeEntityId(row.createdAt),
        worldId: normalizeEntityId(row.worldId),
        groupName: normalizeGroupName(row.groupName)
    };
}

function normalizeAvatarFavoriteRow(row: FavoriteRow): AvatarFavoriteRow {
    return {
        created_at: normalizeEntityId(row.createdAt),
        avatarId: normalizeEntityId(row.avatarId),
        groupName: normalizeGroupName(row.groupName)
    };
}

function normalizeFriendFavoriteRow(row: FavoriteRow): FriendFavoriteRow {
    return {
        created_at: normalizeEntityId(row.createdAt),
        userId: normalizeEntityId(row.userId),
        groupName: normalizeGroupName(row.groupName)
    };
}

function normalizeEntityId(value: unknown) {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeGroupName(value: unknown) {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeGroupList(values: unknown) {
    return Array.from(
        new Set(
            (Array.isArray(values) ? values : [])
                .map(normalizeGroupName)
                .filter(Boolean)
        )
    ).sort((left, right) => left.localeCompare(right));
}

async function getExplicitLocalFavoriteGroups(
    kind: unknown,
    currentUserId?: unknown
) {
    const key = getLocalFavoriteGroupConfigKey(kind);
    if (!key) {
        return [];
    }

    if (kind !== 'friend') {
        return normalizeGroupList(await configRepository.getArray(key, []));
    }

    const normalizedUserId = normalizeEntityId(currentUserId);
    const [sharedGroups, accountGroups] = await Promise.all([
        configRepository.getArray(key, []),
        normalizedUserId
            ? configRepository.getArray(`${key}:${normalizedUserId}`, [])
            : Promise.resolve([])
    ]);
    return normalizeGroupList([
        ...(sharedGroups ?? []),
        ...(accountGroups ?? [])
    ]);
}

async function createLocalFavoriteGroup({
    kind,
    groupName
}: LocalFavoriteGroupInput) {
    const key = getLocalFavoriteGroupConfigKey(kind);
    const normalizedGroupName = normalizeGroupName(groupName);
    if (!key || !normalizedGroupName) {
        throw new Error(
            'LocalFavoritesRepository.createLocalFavoriteGroup requires kind and groupName.'
        );
    }
    if (!isLocalFavoriteKind(kind)) {
        throw new Error('Local favorite kind is invalid.');
    }

    const input = {
        kind,
        groupName: normalizedGroupName
    } satisfies IpcLocalFavoriteGroupInput;

    applyLocalFavoriteGroupWrite(
        await commands.appLocalFavoriteGroupCreate(input)
    );
}

async function getWorldFavorites() {
    return (await commands.appFavoriteList('world')).map(
        normalizeWorldFavoriteRow
    );
}

async function getAvatarFavorites() {
    return (await commands.appFavoriteList('avatar')).map(
        normalizeAvatarFavoriteRow
    );
}

async function getFriendFavorites() {
    return (await commands.appFavoriteList('friend')).map(
        normalizeFriendFavoriteRow
    );
}

async function addLocalFavorite({
    kind,
    entityId,
    groupName
}: LocalFavoriteInput) {
    const validKind = isLocalFavoriteKind(kind);
    const normalizedEntityId = normalizeEntityId(entityId);
    const normalizedGroupName = normalizeGroupName(groupName);

    if (!validKind || !normalizedEntityId || !normalizedGroupName) {
        throw new Error(
            'LocalFavoritesRepository.addLocalFavorite requires kind, entityId, and groupName.'
        );
    }

    const input = {
        kind,
        entityId: normalizedEntityId,
        groupName: normalizedGroupName
    } satisfies IpcLocalFavoriteInput;

    return commands.appLocalFavoriteAdd(input);
}

function addAvatarToFavorites(avatarId: unknown, groupName: unknown) {
    return addLocalFavorite({
        kind: 'avatar',
        entityId: avatarId,
        groupName
    });
}

function addWorldToFavorites(worldId: unknown, groupName: unknown) {
    return addLocalFavorite({
        kind: 'world',
        entityId: worldId,
        groupName
    });
}

function addFriendToLocalFavorites(userId: unknown, groupName: unknown) {
    return addLocalFavorite({
        kind: 'friend',
        entityId: userId,
        groupName
    });
}

async function removeLocalFavorite({
    kind,
    entityId,
    groupName
}: LocalFavoriteInput) {
    const validKind = isLocalFavoriteKind(kind);
    const normalizedEntityId = normalizeEntityId(entityId);
    const normalizedGroupName = normalizeEntityId(groupName);

    if (!validKind || !normalizedEntityId || !normalizedGroupName) {
        throw new Error(
            'LocalFavoritesRepository.removeLocalFavorite requires kind, entityId, and groupName.'
        );
    }

    const input = {
        kind,
        entityId: normalizedEntityId,
        groupName: normalizedGroupName
    } satisfies IpcLocalFavoriteInput;

    return commands.appLocalFavoriteRemove(input);
}

async function renameLocalFavoriteGroup({
    kind,
    groupName,
    newGroupName
}: RenameLocalFavoriteGroupInput) {
    const validKind = isLocalFavoriteKind(kind);
    const normalizedGroupName = normalizeGroupName(groupName);
    const normalizedNewGroupName = normalizeGroupName(newGroupName);

    if (!validKind || !normalizedGroupName || !normalizedNewGroupName) {
        throw new Error(
            'LocalFavoritesRepository.renameLocalFavoriteGroup requires kind, groupName, and newGroupName.'
        );
    }

    const input = {
        kind,
        groupName: normalizedGroupName,
        newGroupName: normalizedNewGroupName
    } satisfies IpcLocalFavoriteGroupRenameInput;

    const write = await commands.appLocalFavoriteGroupRename(input);
    applyLocalFavoriteGroupWrite(write);
    return write.affected;
}

async function deleteLocalFavoriteGroup({
    kind,
    groupName
}: LocalFavoriteGroupInput) {
    const validKind = isLocalFavoriteKind(kind);
    const normalizedGroupName = normalizeGroupName(groupName);

    if (!validKind || !normalizedGroupName) {
        throw new Error(
            'LocalFavoritesRepository.deleteLocalFavoriteGroup requires kind and groupName.'
        );
    }

    const input = {
        kind,
        groupName: normalizedGroupName
    } satisfies IpcLocalFavoriteGroupInput;

    const write = await commands.appLocalFavoriteGroupDelete(input);
    applyLocalFavoriteGroupWrite(write);
    return write.affected;
}

const favoritePersistenceRepository = Object.freeze({
    addAvatarToFavorites,
    addFriendToLocalFavorites,
    addWorldToFavorites,
    getExplicitLocalFavoriteGroups,
    createLocalFavoriteGroup,
    getWorldFavorites,
    getAvatarFavorites,
    getFriendFavorites,
    addLocalFavorite,
    removeLocalFavorite,
    renameLocalFavoriteGroup,
    deleteLocalFavoriteGroup
});

export {
    addAvatarToFavorites,
    addFriendToLocalFavorites,
    addWorldToFavorites,
    getExplicitLocalFavoriteGroups,
    createLocalFavoriteGroup,
    getWorldFavorites,
    getAvatarFavorites,
    getFriendFavorites,
    addLocalFavorite,
    removeLocalFavorite,
    renameLocalFavoriteGroup,
    deleteLocalFavoriteGroup
};
export default favoritePersistenceRepository;
