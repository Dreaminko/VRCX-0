import { beforeEach, describe, expect, it, vi } from 'vitest';

const tauriMock = vi.hoisted(() => ({
    appIngestUserFacts: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: tauriMock
}));

import {
    flushPendingUserFactEntries,
    ingestUserFactEntries,
    resetPendingUserFactEntries
} from './userFactAccessService';

describe('userFactAccessService', () => {
    beforeEach(() => {
        resetPendingUserFactEntries();
        tauriMock.appIngestUserFacts.mockReset();
        tauriMock.appIngestUserFacts.mockResolvedValue(undefined);
    });

    it('batches one tick while merging only equivalent user sources', async () => {
        ingestUserFactEntries([
            {
                user: {
                    id: 'usr_target',
                    displayName: 'Target',
                    tags: ['system_trust_known']
                },
                source: 'profile'
            }
        ]);
        ingestUserFactEntries([
            {
                user: {
                    id: 'usr_target',
                    displayName: '',
                    currentAvatarImageUrl: 'https://example.test/avatar.png',
                    tags: []
                },
                source: 'profile'
            },
            {
                user: { id: 'usr_target', location: 'wrld_live:123' },
                source: 'realtime',
                isFriend: true,
                stateBucket: 'online'
            }
        ]);

        await flushPendingUserFactEntries();

        expect(tauriMock.appIngestUserFacts).toHaveBeenCalledTimes(1);
        expect(tauriMock.appIngestUserFacts).toHaveBeenCalledWith([
            {
                user: {
                    id: 'usr_target',
                    displayName: 'Target',
                    currentAvatarImageUrl: 'https://example.test/avatar.png',
                    tags: ['system_trust_known']
                },
                source: 'profile'
            },
            {
                user: { id: 'usr_target', location: 'wrld_live:123' },
                source: 'realtime',
                isFriend: true,
                stateBucket: 'online'
            }
        ]);
    });

    it('keeps an empty tags array when it is the only value to ingest', async () => {
        ingestUserFactEntries([
            {
                user: { id: 'usr_target', tags: [] },
                source: 'profile'
            }
        ]);

        await flushPendingUserFactEntries();

        expect(tauriMock.appIngestUserFacts).toHaveBeenCalledWith([
            {
                user: { id: 'usr_target', tags: [] },
                source: 'profile'
            }
        ]);
    });
});
