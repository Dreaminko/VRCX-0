import { commands } from '@/platform/tauri/bindings';

type MutualGraphMeta = {
    lastFetchedAt: string | null;
    optedOut: boolean;
};

async function getSnapshot(userId: unknown): Promise<{
    snapshot: Map<string, string[]>;
    meta: Map<string, MutualGraphMeta>;
}> {
    const {
        friendIds,
        links,
        meta: metaRows
    } = await commands.appMutualGraphSnapshotGet(
        typeof userId === 'string' ? userId.trim() : String(userId ?? '').trim()
    );

    const snapshot = new Map<string, string[]>();
    const meta = new Map<string, MutualGraphMeta>();

    for (const friendId of friendIds) {
        const normalizedFriendId = String(friendId || '');
        if (normalizedFriendId && !snapshot.has(normalizedFriendId)) {
            snapshot.set(normalizedFriendId, []);
        }
    }

    for (const row of links) {
        const friendId = row.friendId;
        const mutualId = row.mutualId;
        if (!friendId || !mutualId) {
            continue;
        }

        const normalizedFriendId = String(friendId);
        const mutualIds = snapshot.get(normalizedFriendId) ?? [];
        mutualIds.push(String(mutualId));
        snapshot.set(normalizedFriendId, mutualIds);
    }

    for (const row of metaRows) {
        const friendId = row.friendId;
        if (!friendId) {
            continue;
        }

        meta.set(String(friendId), {
            lastFetchedAt: String(row.lastFetchedAt || '') || null,
            optedOut: Boolean(row.optedOut)
        });
    }

    return {
        snapshot,
        meta
    };
}

const mutualGraphPersistenceRepository = Object.freeze({
    getSnapshot
});

export { getSnapshot };
export default mutualGraphPersistenceRepository;
