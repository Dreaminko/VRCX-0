import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    ensureUserTables: vi.fn(),
    getInt: vi.fn(),
    queryFeedReadModel: vi.fn()
}));

vi.mock('./configRepository', () => ({
    default: {
        getInt: mocks.getInt
    }
}));

vi.mock('./feedPersistenceRepository', () => ({
    default: {
        queryFeedReadModel: mocks.queryFeedReadModel
    }
}));

vi.mock('./userSessionRepository', () => ({
    default: {
        ensureUserTables: mocks.ensureUserTables
    }
}));

import feedRepository from './feedRepository';

describe('feedRepository', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.getInt.mockImplementation((key: string) =>
            Promise.resolve(key === 'searchLimit' ? 50_000 : 500)
        );
        mocks.ensureUserTables.mockResolvedValue({
            userId: 'usr_feed_limit',
            userPrefix: 'usrfeedlimit'
        });
        mocks.queryFeedReadModel.mockResolvedValue({
            rows: [],
            maxSequence: 0
        });
    });

    it('honors an explicit persistence read limit', async () => {
        await feedRepository.queryFeedReadModel({
            userId: 'usr_feed_limit',
            maxEntries: 80,
            maxRows: 80
        });

        expect(mocks.queryFeedReadModel).toHaveBeenCalledWith(
            expect.objectContaining({
                maxEntries: 80,
                maxRows: 80
            })
        );
    });
});
