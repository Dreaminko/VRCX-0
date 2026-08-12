export type FeedCursor = {
    createdAt: string;
    sourceRank: number;
    rowId: number;
};

export type FeedReadModelResult<TRow = Record<string, unknown>> = {
    rows: TRow[];
    maxSequence: number;
    persistedCursor?: FeedCursor | null;
    persistedHasMore?: boolean;
};
