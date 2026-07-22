import type { FriendRosterById } from '@/domain/friends/friendRosterTypes';

import { mutualFriendUsername } from './mutualFriendsGraphData';
import {
    isValidMutualFriendId,
    MUTUAL_GRAPH_PICKER_RESULT_LIMIT,
    normalizeMutualFriendId
} from './mutualFriendsSettings';
import type {
    MutualFriendPickerOption,
    MutualFriendSnapshot
} from './mutualFriendsTypes';

export function truncateMutualFriendLabel(value: string, maxLength = 18) {
    const text = String(value || '');
    return text.length <= maxLength
        ? text
        : `${text.slice(0, Math.max(0, maxLength - 1))}…`;
}

export function mutualFriendPickerOptionMatches(
    option: MutualFriendPickerOption | null | undefined,
    query: string
) {
    const normalizedQuery = String(query || '')
        .trim()
        .toLowerCase();
    if (!normalizedQuery) {
        return true;
    }
    const text = [
        option?.label,
        option?.displayLabel,
        option?.value,
        option?.search,
        option?.user?.displayName,
        mutualFriendUsername(option?.user)
    ]
        .filter(Boolean)
        .join(' ')
        .toLowerCase();
    return normalizedQuery
        .split(/\s+/)
        .filter(Boolean)
        .every((token) => text.includes(token));
}

export function filterMutualFriendPickerOptions(
    options: MutualFriendPickerOption[] | null | undefined,
    query: string,
    limit: number = MUTUAL_GRAPH_PICKER_RESULT_LIMIT,
    selectedIds: readonly string[] | Set<string> | null = null
) {
    const selectedIdSet = new Set(
        (selectedIds ? [...selectedIds] : [])
            .map(normalizeMutualFriendId)
            .filter(Boolean)
    );

    return (Array.isArray(options) ? options : [])
        .filter((option) => mutualFriendPickerOptionMatches(option, query))
        .sort((left, right) => {
            const leftSelected = selectedIdSet.has(
                normalizeMutualFriendId(left?.value)
            );
            const rightSelected = selectedIdSet.has(
                normalizeMutualFriendId(right?.value)
            );
            if (leftSelected !== rightSelected) {
                return leftSelected ? -1 : 1;
            }
            return 0;
        })
        .slice(0, limit);
}

export function buildMutualFriendPickerOption(
    userId: unknown,
    friendsById: FriendRosterById,
    fallbackName = '',
    degree?: number
): MutualFriendPickerOption | null {
    const normalizedId = normalizeMutualFriendId(userId);
    if (!isValidMutualFriendId(normalizedId)) {
        return null;
    }
    const user = friendsById[normalizedId] ?? null;
    const label =
        user?.displayName ||
        mutualFriendUsername(user) ||
        fallbackName ||
        'User';
    return {
        value: normalizedId,
        label,
        displayLabel: Number.isFinite(degree) ? `${label} (${degree})` : label,
        search: `${label} ${normalizedId}`,
        user,
        degree
    };
}

export function buildMutualFriendExcludePickerOptions(
    snapshot: MutualFriendSnapshot | null | undefined,
    friendsById: FriendRosterById,
    currentUserId: string
) {
    const seen = new Set<string>();
    const items: MutualFriendPickerOption[] = [];

    function pushOption(userId: unknown, fallbackName = '') {
        const normalizedId = normalizeMutualFriendId(userId);
        if (
            !isValidMutualFriendId(normalizedId) ||
            normalizedId === currentUserId ||
            seen.has(normalizedId)
        ) {
            return;
        }
        const option = buildMutualFriendPickerOption(
            normalizedId,
            friendsById,
            fallbackName
        );
        if (option) {
            seen.add(normalizedId);
            items.push(option);
        }
    }

    if (snapshot instanceof Map) {
        snapshot.forEach((mutualIds, friendId) => {
            pushOption(friendId);
            for (const mutualId of Array.isArray(mutualIds) ? mutualIds : []) {
                pushOption(mutualId);
            }
        });
    }

    return items.sort((left, right) => left.label.localeCompare(right.label));
}
