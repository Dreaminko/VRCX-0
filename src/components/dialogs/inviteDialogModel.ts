import type {
    FriendRecordInput,
    FriendRosterInputById
} from '@/domain/friends/friendRosterTypes';
import { normalizeString as normalizeId } from '@/shared/utils/string';

type InviteCurrentUser = FriendRecordInput | null | undefined;

type InviteFavoriteInputs = {
    favoriteFriendGroups?: unknown;
    groupedFavoriteFriendIdsByGroupKey?: Record<string, unknown> | null;
    localFriendFavoriteGroups?: unknown;
    localFriendFavorites?: Record<string, unknown> | null;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

export function onlineFriendIdsFromGroup(
    userIds: unknown,
    friendsById: FriendRosterInputById
) {
    return (Array.isArray(userIds) ? userIds : [])
        .map(normalizeId)
        .filter((userId, index, source) => {
            const friend = friendsById[userId];
            return (
                userId &&
                source.indexOf(userId) === index &&
                (friend?.stateBucket === 'online' || friend?.state === 'online')
            );
        });
}

export function displayNameForUser(
    userId: string,
    friendsById: FriendRosterInputById,
    currentUser: InviteCurrentUser
) {
    if (normalizeId(currentUser?.id) === userId) {
        return (
            normalizeId(currentUser?.displayName) ||
            normalizeId(currentUser?.username) ||
            userId
        );
    }
    const friend = friendsById[userId];
    const ref = isRecord(friend?.ref) ? friend.ref : friend;
    return (
        normalizeId(ref?.displayName) ||
        normalizeId(ref?.username) ||
        normalizeId(friend?.name) ||
        userId
    );
}

export function pushUniqueLabel(labels: string[], label: unknown) {
    const normalizedLabel = normalizeId(label);
    if (normalizedLabel && !labels.includes(normalizedLabel)) {
        labels.push(normalizedLabel);
    }
}

export function filterInviteUserIds({
    selectableUserIds,
    search,
    friendsById,
    currentUser
}: {
    selectableUserIds: string[];
    search: string;
    friendsById: FriendRosterInputById;
    currentUser: InviteCurrentUser;
}) {
    const query = search.trim().toLowerCase();
    if (!query) {
        return selectableUserIds;
    }
    return selectableUserIds.filter((userId) => {
        const displayName = displayNameForUser(
            userId,
            friendsById,
            currentUser
        );
        return (
            userId.toLowerCase().includes(query) ||
            displayName.toLowerCase().includes(query)
        );
    });
}

export function sortInviteUserIdsWithSelectedFirst(
    filteredUserIds: string[],
    selectedUserIdSet: ReadonlySet<string>
) {
    return [...filteredUserIds].sort((left, right) => {
        const leftSelected = selectedUserIdSet.has(normalizeId(left));
        const rightSelected = selectedUserIdSet.has(normalizeId(right));
        if (leftSelected !== rightSelected) {
            return leftSelected ? -1 : 1;
        }
        return 0;
    });
}

export function buildFavoriteGroupLabelsByUserId({
    favoriteFriendGroups,
    groupedFavoriteFriendIdsByGroupKey,
    localFriendFavoriteGroups,
    localFriendFavorites
}: InviteFavoriteInputs) {
    const labelsByUserId: Record<string, string[]> = {};
    function addLabel(userId: unknown, label: unknown) {
        const normalizedUserId = normalizeId(userId);
        if (!normalizedUserId) {
            return;
        }
        if (!labelsByUserId[normalizedUserId]) {
            labelsByUserId[normalizedUserId] = [];
        }
        pushUniqueLabel(labelsByUserId[normalizedUserId], label);
    }

    for (const group of Array.isArray(favoriteFriendGroups)
        ? favoriteFriendGroups
        : []) {
        const groupRecord = isRecord(group) ? group : {};
        const key = normalizeId(groupRecord.key);
        const label = normalizeId(groupRecord.displayName) || key;
        for (const userId of Array.isArray(
            groupedFavoriteFriendIdsByGroupKey?.[key]
        )
            ? groupedFavoriteFriendIdsByGroupKey[key]
            : []) {
            addLabel(userId, label);
        }
    }

    for (const groupName of Array.isArray(localFriendFavoriteGroups)
        ? localFriendFavoriteGroups
        : Object.keys(localFriendFavorites || {})) {
        const key = normalizeId(groupName);
        for (const userId of Array.isArray(localFriendFavorites?.[key])
            ? localFriendFavorites[key]
            : []) {
            addLabel(userId, key);
        }
    }

    return labelsByUserId;
}

export function buildFriendsInCurrentInstanceIds({
    currentLocationPlayerIds,
    friendsById
}: {
    currentLocationPlayerIds: unknown;
    friendsById: FriendRosterInputById;
}) {
    const ids = new Set(
        (Array.isArray(currentLocationPlayerIds)
            ? currentLocationPlayerIds
            : []
        ).map(normalizeId)
    );
    return [...ids].filter((userId) => userId && friendsById[userId]);
}

export function buildFavoriteGroupItems({
    favoriteFriendGroups,
    groupedFavoriteFriendIdsByGroupKey,
    localFriendFavoriteGroups,
    localFriendFavorites,
    friendsById
}: InviteFavoriteInputs & { friendsById: FriendRosterInputById }) {
    const remote = (
        Array.isArray(favoriteFriendGroups) ? favoriteFriendGroups : []
    )
        .map((group) => {
            const groupRecord = isRecord(group) ? group : {};
            const key = normalizeId(groupRecord.key);
            const userIds = onlineFriendIdsFromGroup(
                groupedFavoriteFriendIdsByGroupKey?.[key],
                friendsById
            );
            return {
                key: `remote:${key}`,
                label: normalizeId(groupRecord.displayName) || key,
                userIds
            };
        })
        .filter((group) => group.key && group.userIds.length);

    const local = (
        Array.isArray(localFriendFavoriteGroups)
            ? localFriendFavoriteGroups
            : []
    )
        .map((groupName) => {
            const key = normalizeId(groupName);
            const userIds = onlineFriendIdsFromGroup(
                localFriendFavorites?.[key],
                friendsById
            );
            return {
                key: `local:${key}`,
                label: key,
                userIds
            };
        })
        .filter((group) => group.key && group.userIds.length);

    return { remote, local };
}
