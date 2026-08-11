import { create } from 'zustand';

import type {
    FeedEntryPatch,
    FeedEntryPatchInput,
    FeedLiveEntry,
    FeedLivePatch,
    FeedLiveEntryPayload
} from '@/domain/feed/feedLiveTypes';
import { normalizeString } from '@/shared/utils/string';
import { usePreferencesStore } from '@/state/preferencesStore';

type FeedLivePushOptions = {
    ownerUserId?: string;
};

interface FeedLiveStoreState {
    version: number;
    entries: FeedLiveEntry[];
    patches: FeedLivePatch[];
    pushEntries: (
        entries:
            | readonly (FeedLiveEntry | null | undefined)[]
            | null
            | undefined,
        options?: FeedLivePushOptions
    ) => void;
    pushPatches: (
        patches:
            | readonly (FeedLivePatch | null | undefined)[]
            | null
            | undefined
    ) => void;
    resetFeedLive: () => void;
    trimEntries: () => void;
}

const initialState: Pick<
    FeedLiveStoreState,
    'version' | 'entries' | 'patches'
> = {
    version: 0,
    entries: [],
    patches: []
};

const PERSISTED_FEED_LIVE_MAX_ENTRIES = 100;

function feedLiveMaxEntries() {
    const preferences = usePreferencesStore.getState();
    return preferences.feedPersistenceDisabled
        ? preferences.tableLimits.maxTableSize
        : PERSISTED_FEED_LIVE_MAX_ENTRIES;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

export function feedEntryCorrectionId(row: FeedLiveEntryPayload): string {
    if (row?.id != null) {
        return `id:${row.id}`;
    }
    const rowId = row?.rowId ?? row?.row_id;
    if (rowId != null) {
        const sourceRank = row?.sourceRank ?? row?.source_rank;
        if (sourceRank != null) {
            return `row:${row?.type ?? ''}:${sourceRank}:${rowId}`;
        }
        return `row:${row?.type ?? ''}:${rowId}`;
    }
    const type = row?.type ?? '';
    const createdAt = row?.created_at ?? row?.createdAt ?? '';
    const userId = row?.userId ?? row?.user_id ?? row?.senderUserId ?? '';
    const details = isRecord(row?.details) ? row.details : {};
    const location = row?.location ?? details.location ?? '';
    const message = row?.message ?? '';
    return `${type}:${createdAt}:${userId}:${location}:${message}`;
}

function nonEmptyFeedPatch(fields: FeedEntryPatchInput): FeedEntryPatch {
    const patch: FeedEntryPatch = {};
    const displayName = normalizeString(fields.displayName);
    if (displayName) {
        patch.displayName = displayName;
    }
    const worldName = normalizeString(fields.worldName);
    if (worldName) {
        patch.worldName = worldName;
    }
    const displayLocation = normalizeString(fields.displayLocation);
    if (displayLocation) {
        patch.displayLocation = displayLocation;
    }
    return patch;
}

export const useFeedLiveStore = create<FeedLiveStoreState>((set) => ({
    ...initialState,
    pushEntries(entries, { ownerUserId = '' }: FeedLivePushOptions = {}) {
        const validEntries = (Array.isArray(entries) ? entries : []).filter(
            (entry): entry is FeedLiveEntry =>
                isRecord(entry) &&
                typeof entry.sequence === 'number' &&
                Number.isFinite(entry.sequence) &&
                entry.sequence > 0 &&
                isRecord(entry.entry)
        );
        if (!validEntries.length) {
            return;
        }
        const maxEntries = feedLiveMaxEntries();
        set((state) => {
            const appended = validEntries
                .filter((entry) => entry.sequence > state.version)
                .map((entry) => ({
                    sequence: entry.sequence,
                    ownerUserId,
                    entry: { ...entry.entry, ownerUserId }
                }));
            if (!appended.length) {
                return state;
            }
            return {
                version: Math.max(
                    state.version,
                    ...appended.map((entry) => entry.sequence)
                ),
                entries: [...state.entries, ...appended].slice(-maxEntries)
            };
        });
    },
    pushPatches(patches) {
        const validPatches = (Array.isArray(patches) ? patches : []).filter(
            (patch): patch is FeedLivePatch =>
                isRecord(patch) &&
                typeof patch.sequence === 'number' &&
                Number.isFinite(patch.sequence) &&
                patch.sequence > 0 &&
                Boolean(normalizeString(patch.id)) &&
                isRecord(patch.fields)
        );
        if (!validPatches.length) {
            return;
        }
        set((state) => {
            const nextPatches = validPatches.filter(
                (patch) => patch.sequence > state.version
            );
            if (!nextPatches.length) {
                return state;
            }
            let entries = state.entries;
            for (const patchEntry of nextPatches) {
                const normalizedId = normalizeString(patchEntry.id);
                const patch = nonEmptyFeedPatch(patchEntry.fields);
                entries = entries.map((entry) => {
                    if (feedEntryCorrectionId(entry.entry) !== normalizedId) {
                        return entry;
                    }
                    return {
                        ...entry,
                        entry: {
                            ...entry.entry,
                            ...patch
                        }
                    };
                });
            }
            const maxEntries = feedLiveMaxEntries();
            return {
                version: Math.max(
                    state.version,
                    ...nextPatches.map((patch) => patch.sequence)
                ),
                entries,
                patches: [...state.patches, ...nextPatches].slice(-maxEntries)
            };
        });
    },
    resetFeedLive() {
        set(initialState);
    },
    trimEntries() {
        const maxEntries = feedLiveMaxEntries();
        set((state) => {
            const entries = state.entries.slice(-maxEntries);
            const patches = state.patches.slice(-maxEntries);
            if (
                entries.length === state.entries.length &&
                patches.length === state.patches.length
            ) {
                return state;
            }
            return {
                entries,
                patches
            };
        });
    }
}));
export type { FeedLiveEntry, FeedLivePatch, FeedLiveStoreState };
