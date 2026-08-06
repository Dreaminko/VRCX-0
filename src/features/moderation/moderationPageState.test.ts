import { describe, expect, it } from 'vitest';

import {
    getModerationRowKey,
    isSameModerationRow,
    MODERATION_DEFAULT_SORTING,
    sanitizeModerationSorting
} from './moderationPageState';

describe('moderationPageState', () => {
    it('distinguishes moderation types that share a remote id', () => {
        const mute = {
            id: 'pmod_shared',
            type: 'mute',
            sourceUserId: 'usr_source',
            targetUserId: 'usr_target',
            created: '2026-07-08T02:07:00.000Z'
        };
        const muteChat = {
            ...mute,
            type: 'muteChat'
        };

        expect(getModerationRowKey(mute)).not.toBe(
            getModerationRowKey(muteChat)
        );
        expect(isSameModerationRow(mute, muteChat)).toBe(false);
    });

    it('drops source and target from saved sorting state', () => {
        expect(
            sanitizeModerationSorting([
                { id: 'sourceDisplayName', desc: false },
                { id: 'created', desc: true },
                { id: 'targetDisplayName', desc: false },
                { id: 'type', desc: false }
            ])
        ).toEqual([
            { id: 'created', desc: true },
            { id: 'type', desc: false }
        ]);

        expect(
            sanitizeModerationSorting([
                { id: 'sourceDisplayName', desc: false },
                { id: 'targetDisplayName', desc: true }
            ])
        ).toBe(MODERATION_DEFAULT_SORTING);
    });
});
