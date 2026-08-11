import type { FeedCursorInput } from '@/platform/tauri/bindings';

export type FeedReadModelResult<TRow = Record<string, unknown>> = {
    rows: TRow[];
    maxSequence: number;
    persistedCursor?: FeedCursorInput | null;
    persistedHasMore?: boolean;
};
