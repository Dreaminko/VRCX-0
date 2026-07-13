import { beforeEach, describe, expect, it } from 'vitest';

import { useWorldCollectionImportStore } from './worldCollectionImportStore';

describe('worldCollectionImportStore', () => {
    beforeEach(() => {
        useWorldCollectionImportStore.getState().finish();
    });

    it('tracks background world collection import progress', () => {
        useWorldCollectionImportStore.getState().start(80);
        useWorldCollectionImportStore.getState().setProgress(12);

        expect(useWorldCollectionImportStore.getState()).toMatchObject({
            active: true,
            progress: 12,
            total: 80
        });

        useWorldCollectionImportStore.getState().finish();
        expect(useWorldCollectionImportStore.getState()).toMatchObject({
            active: false,
            progress: 0,
            total: 0
        });
    });
});
