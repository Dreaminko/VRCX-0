const CURRENT_USER_LOCAL_AUTHORITY_FIELDS = [
    'friends',
    'onlineFriends',
    'activeFriends',
    'offlineFriends',
    'status',
    'statusDescription',
    'state',
    'stateBucket',
    'pendingOffline',
    'location',
    '$location',
    '$location_at',
    'locationUpdatedAt',
    'worldId',
    'instanceId',
    'travelingToLocation',
    'travelingToWorld',
    'travelingToInstance',
    '$travelingToLocation',
    '$travelingToTime',
    'travelingToTime',
    '$previousLocation',
    '$previousLocation_at'
];

const CURRENT_USER_FRIEND_ARRAY_FIELDS = new Set([
    'friends',
    'onlineFriends',
    'activeFriends',
    'offlineFriends'
]);

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function areValuesEqual(left: unknown, right: unknown): boolean {
    if (Object.is(left, right)) {
        return true;
    }

    if (
        (left && typeof left === 'object') ||
        (right && typeof right === 'object')
    ) {
        try {
            return JSON.stringify(left) === JSON.stringify(right);
        } catch {
            return false;
        }
    }

    return false;
}

function hasField(source: unknown, field: string): boolean {
    return (
        isRecord(source) && Object.prototype.hasOwnProperty.call(source, field)
    );
}

function isResponseAuthoritativeField(
    responseUser: Record<string, unknown>,
    field: string,
    responseAuthorityFields?: ReadonlySet<string>
): boolean {
    return Boolean(
        hasField(responseUser, field) &&
        (CURRENT_USER_FRIEND_ARRAY_FIELDS.has(field) ||
            responseAuthorityFields?.has(field))
    );
}

export function mergeCurrentUserResponseSnapshot({
    responseUser,
    baseSnapshot,
    currentSnapshot,
    overlayPatch,
    responseAuthorityFields
}: {
    responseUser: Record<string, unknown>;
    baseSnapshot: Record<string, unknown> | null;
    currentSnapshot: unknown;
    overlayPatch: Record<string, unknown> | null;
    responseAuthorityFields?: ReadonlySet<string>;
}): Record<string, unknown> {
    const currentSnapshotRecord = isRecord(currentSnapshot)
        ? currentSnapshot
        : null;
    const user: Record<string, unknown> = currentSnapshotRecord
        ? { ...currentSnapshotRecord, ...responseUser }
        : { ...responseUser };

    for (const field of CURRENT_USER_LOCAL_AUTHORITY_FIELDS) {
        if (
            isResponseAuthoritativeField(
                responseUser,
                field,
                responseAuthorityFields
            )
        ) {
            continue;
        }
        if (hasField(currentSnapshot, field)) {
            user[field] = currentSnapshotRecord?.[field];
        }
    }

    if (
        baseSnapshot &&
        normalizeString(baseSnapshot.id) ===
            normalizeString(currentSnapshotRecord?.id)
    ) {
        const keys = new Set([
            ...Object.keys(baseSnapshot),
            ...Object.keys(currentSnapshotRecord || {})
        ]);
        keys.delete('id');
        for (const key of keys) {
            if (
                isResponseAuthoritativeField(
                    responseUser,
                    key,
                    responseAuthorityFields
                )
            ) {
                continue;
            }
            if (
                !areValuesEqual(baseSnapshot[key], currentSnapshotRecord?.[key])
            ) {
                user[key] = currentSnapshotRecord?.[key];
            }
        }
    }

    return overlayPatch ? { ...user, ...overlayPatch } : user;
}
import { normalizeString } from './string';
