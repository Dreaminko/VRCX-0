import { commands } from '@/platform/tauri/bindings';
import type {
    AvatarCacheOutput,
    AvatarTagInput,
    AvatarTagsPatchInput
} from '@/platform/tauri/bindings';
import { normalizeString } from '@/shared/utils/string';

type ObjectRow = Record<string, unknown>;

interface AvatarTag {
    tag?: unknown;
    color?: unknown;
}

function asObjectRow(row: ObjectRow | unknown[] | null | undefined): ObjectRow {
    return row && !Array.isArray(row) ? row : {};
}

function parseInteger(value: unknown, fallback: number) {
    return Number.parseInt(String(value ?? fallback), 10) || fallback;
}

function normalizeAvatarCacheRow(
    row: AvatarCacheOutput | ObjectRow | unknown[] | null | undefined
) {
    if (Array.isArray(row)) {
        return {
            id: row[0] ?? '',
            authorId: row[2] ?? '',
            authorName: row[3] ?? '',
            created_at: row[4] ?? '',
            description: row[5] ?? '',
            imageUrl: row[6] ?? '',
            name: row[7] ?? '',
            releaseStatus: row[8] ?? '',
            thumbnailImageUrl: row[9] ?? '',
            updated_at: row[10] ?? '',
            version: row[11] ?? 0
        };
    }

    const record = asObjectRow(row);
    return {
        id: record.id ?? '',
        authorId: record.author_id ?? record.authorId ?? '',
        authorName: record.author_name ?? record.authorName ?? '',
        created_at: record.created_at ?? '',
        description: record.description ?? '',
        imageUrl: record.image_url ?? record.imageUrl ?? '',
        name: record.name ?? '',
        releaseStatus: record.release_status ?? record.releaseStatus ?? '',
        thumbnailImageUrl:
            record.thumbnail_image_url ?? record.thumbnailImageUrl ?? '',
        updated_at: record.updated_at ?? '',
        version: record.version ?? 0
    };
}

function normalizeAvatarTagInput(entry: AvatarTag): AvatarTagInput {
    return {
        tag: normalizeString(entry.tag),
        color: entry.color ?? null
    };
}

async function addAvatarTimeSpent(
    userId: unknown,
    avatarId: unknown,
    timeSpent: unknown
) {
    const normalizedUserId = normalizeString(userId);
    const normalizedAvatarId = normalizeString(avatarId);
    const normalizedTimeSpent = parseInteger(timeSpent, 0);
    if (!normalizedUserId || !normalizedAvatarId) {
        return;
    }

    await commands.appAvatarTimeSpentAdd(
        normalizedUserId,
        normalizedAvatarId,
        normalizedTimeSpent
    );
}

async function getAvatarTimeSpent(userId: unknown, avatarId: unknown) {
    const normalizedUserId = normalizeString(userId);
    const normalizedAvatarId = normalizeString(avatarId);
    const ref = {
        timeSpent: 0,
        avatarId: normalizedAvatarId
    };
    if (!normalizedUserId || !normalizedAvatarId) {
        return ref;
    }

    const row = await commands.appAvatarTimeSpentGet(
        normalizedUserId,
        normalizedAvatarId
    );
    ref.timeSpent = parseInteger(row.timeSpent, 0);
    return ref;
}

async function getAllAvatarTimeSpent(userId: unknown) {
    const map = new Map<string, number>();
    const normalizedUserId = normalizeString(userId);
    if (!normalizedUserId) {
        return map;
    }

    const rows = await commands.appAvatarTimeSpentList(normalizedUserId);
    for (const row of rows) {
        const avatarId = row.avatarId;
        if (avatarId) {
            map.set(avatarId, parseInteger(row.timeSpent, 0));
        }
    }
    return map;
}

async function getAvatarHistory(userId: unknown, limit: unknown = 100) {
    const normalizedUserId = normalizeString(userId);
    if (!normalizedUserId) {
        return [];
    }

    const rows = await commands.appAvatarHistoryList(
        normalizedUserId,
        parseInteger(limit, 100)
    );
    return rows.map(normalizeAvatarCacheRow);
}

async function clearAvatarHistory(userId: unknown) {
    const normalizedUserId = normalizeString(userId);
    if (!normalizedUserId) {
        return;
    }
    await commands.appAvatarHistoryClear(normalizedUserId);
}

async function getAvatarTags(avatarId: unknown) {
    const normalizedAvatarId = normalizeString(avatarId);
    if (!normalizedAvatarId) {
        return [];
    }
    const rows = await commands.appAvatarTagsGet(normalizedAvatarId);
    return rows.map((row) => ({
        tag: row.tag,
        color: row.color || null
    }));
}

async function getAllAvatarTags() {
    const map = new Map<string, AvatarTag[]>();
    const rows = await commands.appAvatarTagsList();
    for (const row of rows) {
        const avatarId = row.avatarId;
        const tag = row.tag;
        const color = row.color || null;
        if (!map.has(avatarId)) {
            map.set(avatarId, []);
        }
        map.get(avatarId)?.push({ tag, color });
    }
    return map;
}

async function getAllDistinctTags() {
    return commands.appAvatarTagsDistinct();
}

async function addAvatarTag(
    avatarId: unknown,
    tag: unknown,
    color: unknown = null
) {
    await commands.appAvatarTagAdd(normalizeString(avatarId), tag, color);
}

async function updateAvatarTagColor(
    avatarId: unknown,
    tag: unknown,
    color: unknown
) {
    await commands.appAvatarTagUpdateColor(
        normalizeString(avatarId),
        tag,
        color
    );
}

async function removeAvatarTag(avatarId: unknown, tag: unknown) {
    await commands.appAvatarTagRemove(normalizeString(avatarId), tag);
}

async function removeAllAvatarTags(avatarId: unknown) {
    await commands.appAvatarTagsRemoveAll(normalizeString(avatarId));
}

async function replaceAvatarTags(avatarId: unknown, entries: AvatarTag[] = []) {
    await commands.appAvatarTagsReplace(
        normalizeString(avatarId),
        (Array.isArray(entries) ? entries : []).map(normalizeAvatarTagInput)
    );
}

async function patchAvatarTags(
    avatarId: unknown,
    previousEntries: AvatarTag[] = [],
    nextEntries: AvatarTag[] = []
) {
    const patch: AvatarTagsPatchInput = {
        previousEntries: (Array.isArray(previousEntries)
            ? previousEntries
            : []
        ).map(normalizeAvatarTagInput),
        nextEntries: (Array.isArray(nextEntries) ? nextEntries : []).map(
            normalizeAvatarTagInput
        )
    };
    await commands.appAvatarTagsPatch(normalizeString(avatarId), patch);
}

const avatarLocalRepository = Object.freeze({
    addAvatarTag,
    addAvatarTimeSpent,
    clearAvatarHistory,
    getAllAvatarTags,
    getAllAvatarTimeSpent,
    getAllDistinctTags,
    getAvatarHistory,
    getAvatarTags,
    getAvatarTimeSpent,
    removeAllAvatarTags,
    removeAvatarTag,
    patchAvatarTags,
    replaceAvatarTags,
    updateAvatarTagColor
});

export {
    addAvatarTag,
    addAvatarTimeSpent,
    clearAvatarHistory,
    getAllAvatarTags,
    getAllAvatarTimeSpent,
    getAllDistinctTags,
    getAvatarHistory,
    getAvatarTags,
    getAvatarTimeSpent,
    removeAllAvatarTags,
    removeAvatarTag,
    patchAvatarTags,
    replaceAvatarTags,
    updateAvatarTagColor
};
export default avatarLocalRepository;
