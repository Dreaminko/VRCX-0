import {
    commands,
    type BrowseHistoryEntityKind,
    type BrowseHistoryPageOutput,
    type BrowseHistoryQueryInput,
    type BrowseHistoryRecordInput
} from '@/platform/tauri/bindings';

export type {
    BrowseHistoryCursor,
    BrowseHistoryEntityKind,
    BrowseHistoryItemOutput,
    BrowseHistoryPageOutput
} from '@/platform/tauri/bindings';

export const browseHistoryRepository = {
    record(input: BrowseHistoryRecordInput): Promise<null> {
        return commands.appBrowseHistoryRecord(input);
    },

    query(input: BrowseHistoryQueryInput): Promise<BrowseHistoryPageOutput> {
        return commands.appBrowseHistoryQuery(input);
    },

    delete(
        ownerUserId: string,
        entityKind: BrowseHistoryEntityKind,
        entityId: string
    ): Promise<number> {
        return commands.appBrowseHistoryDelete(
            ownerUserId,
            entityKind,
            entityId
        );
    },

    clear(
        ownerUserId: string,
        entityKind: BrowseHistoryEntityKind | null
    ): Promise<number> {
        return commands.appBrowseHistoryClear(ownerUserId, entityKind);
    },

    getRetentionDays(): Promise<number> {
        return commands.appBrowseHistoryRetentionDaysGet();
    },

    setRetentionDays(retentionDays: number): Promise<number> {
        return commands.appBrowseHistoryRetentionDaysSet(retentionDays);
    }
};
