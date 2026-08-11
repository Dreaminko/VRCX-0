import type { GroupModerationBatchProgress } from '@/platform/tauri/bindings';

interface GroupModerationBatchProgressEvent {
    count: number;
    lastPayload: unknown;
}

function isGroupModerationBatchProgress(
    value: unknown
): value is GroupModerationBatchProgress {
    return Boolean(
        value &&
        typeof value === 'object' &&
        'groupId' in value &&
        typeof value.groupId === 'string' &&
        'ownerUserId' in value &&
        typeof value.ownerUserId === 'string' &&
        'endpoint' in value &&
        typeof value.endpoint === 'string' &&
        'completed' in value &&
        typeof value.completed === 'number' &&
        'total' in value &&
        typeof value.total === 'number'
    );
}

export function resolveGroupModerationBatchProgress({
    busy,
    currentAuthEndpoint,
    currentUserId,
    endpoint,
    event,
    groupId,
    previousEventCount
}: {
    busy: boolean;
    currentAuthEndpoint: string;
    currentUserId: string | null;
    endpoint: string;
    event: GroupModerationBatchProgressEvent | null | undefined;
    groupId: string;
    previousEventCount: number;
}) {
    const progress = event?.lastPayload;
    if (
        !busy ||
        !event ||
        event.count <= previousEventCount ||
        !isGroupModerationBatchProgress(progress) ||
        progress.ownerUserId !== currentUserId ||
        progress.endpoint !== endpoint ||
        currentAuthEndpoint !== endpoint ||
        progress.groupId !== groupId
    ) {
        return null;
    }
    return { current: progress.completed, total: progress.total };
}
