import { describe, expect, it } from 'vitest';

import { resolveGroupModerationBatchProgress } from './groupModerationWorkspaceContext';

const progress = {
    groupId: 'grp_current',
    ownerUserId: 'usr_current',
    endpoint: 'https://api.example.test/api/1',
    completed: 3,
    total: 8
};

describe('resolveGroupModerationBatchProgress', () => {
    it('accepts only a newer event for the active account, endpoint and group', () => {
        expect(
            resolveGroupModerationBatchProgress({
                busy: true,
                currentAuthEndpoint: progress.endpoint,
                currentUserId: progress.ownerUserId,
                endpoint: progress.endpoint,
                event: { count: 5, lastPayload: progress },
                groupId: progress.groupId,
                previousEventCount: 4
            })
        ).toEqual({ current: 3, total: 8 });
    });

    it.each([
        ['idle', { busy: false }],
        ['replayed event', { previousEventCount: 5 }],
        ['different user', { currentUserId: 'usr_other' }],
        ['different auth endpoint', { currentAuthEndpoint: 'https://other' }],
        ['different workspace endpoint', { endpoint: 'https://other' }],
        [
            'different group',
            {
                event: {
                    count: 5,
                    lastPayload: { ...progress, groupId: 'grp_other' }
                }
            }
        ]
    ])('rejects a %s progress update', (_label, overrides) => {
        expect(
            resolveGroupModerationBatchProgress({
                busy: true,
                currentAuthEndpoint: progress.endpoint,
                currentUserId: progress.ownerUserId,
                endpoint: progress.endpoint,
                event: { count: 5, lastPayload: progress },
                groupId: progress.groupId,
                previousEventCount: 4,
                ...overrides
            })
        ).toBeNull();
    });
});
