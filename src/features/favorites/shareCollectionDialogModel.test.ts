import { describe, expect, it } from 'vitest';

import {
    buildShareCollectionWorldIds,
    SHARE_COLLECTION_CLIENT_WORLD_CAP
} from './shareCollectionDialogModel';

describe('buildShareCollectionWorldIds', () => {
    it('keeps wrld ids in order, drops invalid ids, deduplicates, and caps the upload list', () => {
        const ids = Array.from(
            { length: SHARE_COLLECTION_CLIENT_WORLD_CAP + 2 },
            (_, index) =>
                `wrld_${index.toString(16).padStart(8, '0')}-1111-1111-1111-111111111111`
        );

        const result = buildShareCollectionWorldIds([
            { id: ids[1] },
            { id: 'not-world' },
            { id: ids[0] },
            { id: ids[1] },
            ...ids.slice(2).map((id) => ({ id }))
        ]);

        expect(result.worldIds).toHaveLength(SHARE_COLLECTION_CLIENT_WORLD_CAP);
        expect(result.totalWorldIds).toBe(
            SHARE_COLLECTION_CLIENT_WORLD_CAP + 2
        );
        expect(result.truncated).toBe(true);
        expect(result.worldIds[0]).toBe(ids[1]);
        expect(result.worldIds[1]).toBe(ids[0]);
        expect(result.worldIds.at(-1)).toBe(
            ids[SHARE_COLLECTION_CLIENT_WORLD_CAP - 1]
        );
    });
});
