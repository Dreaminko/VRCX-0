import { beforeEach, describe, expect, it, vi } from 'vitest';

const commandMocks = vi.hoisted(() => ({
    appFriendLogHistoryQuery: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({ commands: commandMocks }));

import { getFriendLogHistory } from './friendLogHistoryRepository';

describe('friendLogHistoryRepository', () => {
    beforeEach(() => {
        vi.resetAllMocks();
        commandMocks.appFriendLogHistoryQuery.mockResolvedValue([]);
    });

    it('keeps rename and trust context while restricting history to supported events', async () => {
        commandMocks.appFriendLogHistoryQuery.mockResolvedValueOnce([
            {
                rowId: 1,
                createdAt: '2026-08-10T10:00:00.000Z',
                type: 'DisplayName',
                userId: 'usr_rename',
                displayName: 'New Name',
                previousDisplayName: 'Old Name',
                trustLevel: '',
                previousTrustLevel: '',
                friendNumber: 4
            },
            {
                rowId: 2,
                createdAt: '2026-08-11T10:00:00.000Z',
                type: 'TrustLevel',
                userId: 'usr_trust',
                displayName: 'Trusted Friend',
                previousDisplayName: '',
                trustLevel: 'Trusted User',
                previousTrustLevel: 'Known User',
                friendNumber: 5
            },
            {
                rowId: 3,
                createdAt: '2026-08-12T10:00:00.000Z',
                type: 'Friend',
                userId: '   ',
                displayName: 'Invalid Row',
                previousDisplayName: '',
                trustLevel: '',
                previousTrustLevel: '',
                friendNumber: 0
            }
        ]);

        const rows = await getFriendLogHistory(' owner_user ', {
            targetUserId: ' target_user ',
            types: [
                'Friend',
                ' DisplayName ',
                'unsupported',
                null,
                'TrustLevel'
            ]
        });

        expect(commandMocks.appFriendLogHistoryQuery).toHaveBeenCalledWith({
            userId: 'owner_user',
            targetUserId: 'target_user',
            types: ['Friend', 'DisplayName', 'TrustLevel']
        });
        expect(rows).toEqual([
            {
                rowId: 1,
                created_at: '2026-08-10T10:00:00.000Z',
                type: 'DisplayName',
                userId: 'usr_rename',
                displayName: 'New Name',
                friendNumber: 4,
                previousDisplayName: 'Old Name'
            },
            {
                rowId: 2,
                created_at: '2026-08-11T10:00:00.000Z',
                type: 'TrustLevel',
                userId: 'usr_trust',
                displayName: 'Trusted Friend',
                friendNumber: 5,
                trustLevel: 'Trusted User',
                previousTrustLevel: 'Known User'
            }
        ]);
    });
});
