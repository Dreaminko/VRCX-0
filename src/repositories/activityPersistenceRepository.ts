import { commands } from '@/platform/tauri/bindings';
import type {
    ActivityBucketCacheInput as IpcActivityBucketCacheInput,
    ActivityBucketCacheQueryInput as IpcActivityBucketCacheQueryInput,
    ActivitySelfSourceBoundsOutput as IpcActivitySelfSourceBoundsOutput,
    ActivitySelfSessionsRefreshInput as IpcActivitySelfSessionsRefreshInput,
    ActivityRefreshMode as IpcActivityRefreshMode,
    ActivitySessionOutput as IpcActivitySessionOutput,
    ActivitySyncStateInput as IpcActivitySyncStateInput,
    ActivitySyncStateOutput as IpcActivitySyncStateOutput,
    ActivityViewKind as IpcActivityViewKind
} from '@/platform/tauri/bindings';

export type ActivityViewKind = IpcActivityViewKind;
type ActivitySyncStateRow = IpcActivitySyncStateOutput;
type ActivitySessionRow = IpcActivitySessionOutput;

type ActivitySessionInputLike = {
    start: number;
    end: number;
    isOpenTail?: boolean;
    sourceRevision?: string;
};

interface ActivitySyncStateEntry {
    userId?: unknown;
    updatedAt?: string;
    isSelf?: unknown;
    sourceLastCreatedAt?: string;
    pendingSessionStartAt?: string | number | null;
    cachedRangeDays?: string | number;
}

interface AppendActivitySessionsInput {
    userId?: unknown;
    sessions?: ActivitySessionInputLike[];
    replaceFromStartAt?: number | null;
}

interface ActivityBucketCacheQuery {
    ownerUserId: string;
    targetUserId?: string;
    rangeDays: number;
    viewKind: ActivityViewKind;
    excludeKey?: string;
}

interface ActivityBucketCacheEntry extends ActivityBucketCacheQuery {
    bucketVersion?: number;
    builtFromCursor?: string;
    rawBuckets?: number[];
    normalizedBuckets?: number[];
    summary?: ActivityBucketCacheSummary;
    builtAt?: string;
}

interface ActivitySelfSessionsRefreshRequest {
    userId?: unknown;
    mode: IpcActivityRefreshMode;
    rangeDays?: string | number;
    nowMs?: number;
}

export type ActivityPersistedSession = {
    start: number;
    end: number;
    isOpenTail: boolean;
    sourceRevision: string;
};

export type ActivitySyncState = {
    userId: string;
    updatedAt: string;
    isSelf: boolean;
    sourceLastCreatedAt: string;
    pendingSessionStartAt: string | number | null;
    cachedRangeDays: number;
};

export type ActivityRefreshResult = {
    sync: ActivitySyncState;
    sessions: ActivityPersistedSession[];
    sourceCount: number;
};

export type ActivitySourceBounds = {
    firstCreatedAt: string;
    lastCreatedAt: string;
    count: number;
};

export type ActivityBucketCacheSummary = Record<string, unknown> & {
    filteredEventCount?: number;
    peakDay?: string;
    peakTime?: string;
    bestOverlapTime?: string;
    overlapPercent?: number;
};

export type ActivityBucketCache = {
    ownerUserId: string;
    targetUserId: string;
    rangeDays: number;
    viewKind: ActivityViewKind;
    excludeKey: string;
    bucketVersion: number;
    builtFromCursor: string;
    rawBuckets: number[];
    normalizedBuckets: number[];
    summary: ActivityBucketCacheSummary;
    builtAt: string;
};

const ACTIVITY_VIEW_KIND = Object.freeze({
    ACTIVITY: 'activity',
    OVERLAP: 'overlap'
});

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function normalizeText(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeNumber(value: unknown): number {
    const parsed = Number.parseFloat(String(value ?? ''));
    return Number.isFinite(parsed) ? parsed : 0;
}

function normalizeInteger(value: unknown): number {
    const parsed = Number.parseInt(String(value ?? ''), 10);
    return Number.isFinite(parsed) ? parsed : 0;
}

function normalizePendingSessionStartAt(
    value: unknown
): string | number | null {
    return typeof value === 'string' || typeof value === 'number'
        ? value
        : null;
}

function normalizeNumberArray(value: unknown): number[] {
    if (Array.isArray(value)) {
        return value.map(normalizeNumber);
    }
    if (typeof value === 'string' && value.trim()) {
        try {
            const parsed = JSON.parse(value) as unknown;
            return Array.isArray(parsed) ? parsed.map(normalizeNumber) : [];
        } catch {
            return [];
        }
    }
    return [];
}

function parseMaybeObject(value: unknown): Record<string, unknown> {
    if (isRecord(value)) {
        return value;
    }
    if (typeof value === 'string' && value.trim()) {
        try {
            const parsed = JSON.parse(value) as unknown;
            return isRecord(parsed) ? parsed : {};
        } catch {
            return {};
        }
    }
    return {};
}

function normalizeBucketSummary(value: unknown): ActivityBucketCacheSummary {
    const source = parseMaybeObject(value);
    const summary: ActivityBucketCacheSummary = { ...source };

    if ('filteredEventCount' in source) {
        summary.filteredEventCount = normalizeInteger(
            source.filteredEventCount
        );
    }
    if ('peakDay' in source) {
        summary.peakDay = normalizeText(source.peakDay);
    }
    if ('peakTime' in source) {
        summary.peakTime = normalizeText(source.peakTime);
    }
    if ('bestOverlapTime' in source) {
        summary.bestOverlapTime = normalizeText(source.bestOverlapTime);
    }
    if ('overlapPercent' in source) {
        summary.overlapPercent = normalizeNumber(source.overlapPercent);
    }

    return summary;
}

function normalizeActivitySyncStateRow(
    row: ActivitySyncStateRow | null,
    fallbackUserId: string
): ActivitySyncState | null {
    if (!row || typeof row !== 'object') {
        return null;
    }

    return {
        userId: normalizeText(row.userId || fallbackUserId),
        updatedAt: normalizeText(row.updatedAt),
        isSelf: Boolean(row.isSelf),
        sourceLastCreatedAt: normalizeText(row.sourceLastCreatedAt),
        pendingSessionStartAt: normalizePendingSessionStartAt(
            row.pendingSessionStartAt
        ),
        cachedRangeDays: normalizeInteger(row.cachedRangeDays)
    };
}

function normalizeActivitySessionRow(
    row: ActivitySessionRow | null
): ActivityPersistedSession | null {
    if (!row || typeof row !== 'object') {
        return null;
    }

    return {
        start: normalizeInteger(row.start),
        end: normalizeInteger(row.end),
        isOpenTail: Boolean(row.isOpenTail),
        sourceRevision: normalizeText(row.sourceRevision)
    };
}

async function getActivitySyncState(
    userId: unknown
): Promise<ActivitySyncState | null> {
    const normalizedUserId = normalizeText(userId);
    if (!normalizedUserId) {
        return null;
    }

    const row = await commands.appActivitySyncStateGet(normalizedUserId);

    if (!row) {
        return null;
    }

    return normalizeActivitySyncStateRow(row, normalizedUserId);
}

async function upsertActivitySyncState(entry: ActivitySyncStateEntry) {
    const normalizedUserId = normalizeText(entry?.userId);
    if (!normalizedUserId) {
        throw new Error(
            'ActivityRepository.upsertActivitySyncState requires a user id.'
        );
    }

    const input = {
        userId: normalizedUserId,
        updatedAt: entry.updatedAt || '',
        isSelf: Boolean(entry.isSelf),
        sourceLastCreatedAt: entry.sourceLastCreatedAt || '',
        pendingSessionStartAt: entry.pendingSessionStartAt ?? null,
        cachedRangeDays:
            Number.parseInt(String(entry.cachedRangeDays ?? 0), 10) || 0
    } satisfies IpcActivitySyncStateInput;

    await commands.appActivitySyncStateUpsert(input);
}

async function refreshSelfActivitySessions({
    userId,
    mode,
    rangeDays = 0,
    nowMs
}: ActivitySelfSessionsRefreshRequest): Promise<ActivityRefreshResult> {
    const normalizedUserId = normalizeText(userId);
    if (!normalizedUserId) {
        throw new Error(
            'ActivityRepository.refreshSelfActivitySessions requires a user id.'
        );
    }

    const input = {
        userId: normalizedUserId,
        mode,
        rangeDays,
        ...(Number.isFinite(nowMs) ? { nowMs } : {})
    } satisfies IpcActivitySelfSessionsRefreshInput;
    const result = await commands.appActivitySelfSessionsRefresh(input);
    const sync = normalizeActivitySyncStateRow(
        result?.sync || null,
        normalizedUserId
    );
    const sessions = Array.isArray(result?.sessions)
        ? result.sessions
              .map(normalizeActivitySessionRow)
              .filter(
                  (row): row is ActivityPersistedSession =>
                      Number.isFinite(row?.start) && Number.isFinite(row?.end)
              )
        : [];

    return {
        sync: sync || {
            userId: normalizedUserId,
            updatedAt: '',
            isSelf: true,
            sourceLastCreatedAt: '',
            pendingSessionStartAt: null,
            cachedRangeDays: 0
        },
        sessions,
        sourceCount: normalizeInteger(result?.sourceCount)
    };
}

async function getActivitySessions(
    userId: unknown
): Promise<ActivityPersistedSession[]> {
    const normalizedUserId = normalizeText(userId);
    if (!normalizedUserId) {
        return [];
    }

    const rows = await commands.appActivitySessionsGet(normalizedUserId);

    if (!Array.isArray(rows)) {
        return [];
    }

    return rows
        .map(normalizeActivitySessionRow)
        .filter(
            (row): row is ActivityPersistedSession =>
                Number.isFinite(row?.start) && Number.isFinite(row?.end)
        );
}

async function getSelfActivitySourceBounds(): Promise<ActivitySourceBounds> {
    const row: IpcActivitySelfSourceBoundsOutput =
        await commands.appActivitySelfSourceBounds();
    return {
        firstCreatedAt: normalizeText(row.firstCreatedAt),
        lastCreatedAt: normalizeText(row.lastCreatedAt),
        count: normalizeInteger(row.count)
    };
}

async function replaceActivitySessions(
    userId: unknown,
    sessions: ActivitySessionInputLike[] = []
) {
    const normalizedUserId = normalizeText(userId);

    await commands.appActivitySessionsReplace(
        normalizedUserId,
        Array.isArray(sessions) ? sessions : []
    );
}

async function appendActivitySessions({
    userId,
    sessions = [],
    replaceFromStartAt = null
}: AppendActivitySessionsInput) {
    const normalizedUserId = normalizeText(userId);

    await commands.appActivitySessionsAppend(
        normalizedUserId,
        Array.isArray(sessions) ? sessions : [],
        replaceFromStartAt !== null && replaceFromStartAt !== undefined
            ? replaceFromStartAt
            : null
    );
}

async function getActivityBucketCache({
    ownerUserId,
    targetUserId = '',
    rangeDays,
    viewKind,
    excludeKey = ''
}: ActivityBucketCacheQuery): Promise<ActivityBucketCache | null> {
    const query = {
        ownerUserId,
        targetUserId,
        rangeDays,
        viewKind,
        excludeKey
    } satisfies IpcActivityBucketCacheQueryInput;
    const row = await commands.appActivityBucketCacheGet(query);
    if (!row) {
        return null;
    }
    return {
        ownerUserId: normalizeText(row.ownerUserId),
        targetUserId: normalizeText(row.targetUserId),
        rangeDays: normalizeInteger(row.rangeDays),
        viewKind: row.viewKind,
        excludeKey: normalizeText(row.excludeKey),
        bucketVersion: normalizeInteger(row.bucketVersion),
        builtFromCursor: normalizeText(row.builtFromCursor),
        rawBuckets: normalizeNumberArray(row.rawBuckets),
        normalizedBuckets: normalizeNumberArray(row.normalizedBuckets),
        summary: normalizeBucketSummary(row.summary),
        builtAt: normalizeText(row.builtAt)
    };
}

async function upsertActivityBucketCache(entry: ActivityBucketCacheEntry) {
    const input = {
        ownerUserId: entry.ownerUserId,
        targetUserId: entry.targetUserId || '',
        rangeDays: entry.rangeDays,
        viewKind: entry.viewKind,
        excludeKey: entry.excludeKey || '',
        bucketVersion: entry.bucketVersion || 1,
        builtFromCursor: entry.builtFromCursor || '',
        rawBuckets: entry.rawBuckets || [],
        normalizedBuckets: entry.normalizedBuckets || [],
        summary: entry.summary || {},
        builtAt: entry.builtAt || ''
    } satisfies IpcActivityBucketCacheInput;

    await commands.appActivityBucketCacheUpsert(input);
}

const activityPersistenceRepository = Object.freeze({
    ACTIVITY_VIEW_KIND,
    getActivityBucketCache,
    getSelfActivitySourceBounds,
    getActivitySyncState,
    upsertActivitySyncState,
    refreshSelfActivitySessions,
    getActivitySessions,
    replaceActivitySessions,
    appendActivitySessions,
    upsertActivityBucketCache
});

export {
    ACTIVITY_VIEW_KIND,
    getActivityBucketCache,
    getSelfActivitySourceBounds,
    getActivitySyncState,
    upsertActivitySyncState,
    refreshSelfActivitySessions,
    getActivitySessions,
    replaceActivitySessions,
    appendActivitySessions,
    upsertActivityBucketCache
};
export default activityPersistenceRepository;
