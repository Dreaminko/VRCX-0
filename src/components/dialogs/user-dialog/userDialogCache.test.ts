import { beforeEach, describe, expect, it } from 'vitest';

import {
    cachePreviousInstances,
    cacheUserStats,
    clearUserDialogCaches,
    dialogTargetKey,
    readCachedPreviousInstances,
    readCachedUserStats,
    type UserDialogPreviousInstance
} from './userDialogCache';

function previousInstance(location: string): UserDialogPreviousInstance {
    return {
        createdAt: '',
        events: [],
        groupName: '',
        lastTs: 0,
        location,
        time: 0,
        worldName: ''
    };
}

describe('userDialogCache', () => {
    beforeEach(() => {
        clearUserDialogCaches();
    });

    it('uses the API endpoint and normalized user id to remember one user dialog target', () => {
        expect(
            dialogTargetKey(' https://api.example.test ', ' usr_target ')
        ).toBe('https://api.example.test:usr_target');
        expect(dialogTargetKey('https://api.example.test', '')).toBe('');
    });

    it('shows empty stats and no instance history before a user has loaded', () => {
        const key = dialogTargetKey('https://api.example.test', 'usr_missing');

        expect(readCachedUserStats(key)).toEqual({
            timeSpent: 0,
            lastSeen: '',
            friendedAt: '',
            joinCount: 0,
            previousDisplayNames: []
        });
        expect(readCachedPreviousInstances(key)).toEqual([]);
    });

    it('restores cached stats and instance history when the same user dialog opens again', () => {
        const key = dialogTargetKey('https://api.example.test', 'usr_target');
        cacheUserStats(key, {
            timeSpent: 12345,
            lastSeen: '2026-01-02T03:04:05.000Z',
            joinCount: 7,
            previousDisplayNames: [{ displayName: 'Old Name' }]
        });
        cachePreviousInstances(key, [
            previousInstance('wrld_one:1'),
            previousInstance('wrld_two:2')
        ]);

        expect(readCachedUserStats(key)).toEqual({
            timeSpent: 12345,
            lastSeen: '2026-01-02T03:04:05.000Z',
            friendedAt: '',
            joinCount: 7,
            previousDisplayNames: [{ displayName: 'Old Name' }]
        });
        expect(readCachedPreviousInstances(key)).toEqual([
            previousInstance('wrld_one:1'),
            previousInstance('wrld_two:2')
        ]);
    });

    it('shows the original cached stats when a dialog-local copy is edited then reopened', () => {
        const key = dialogTargetKey('https://api.example.test', 'usr_target');
        cacheUserStats(key, {
            timeSpent: '2000',
            lastSeen: '2026-01-02T03:04:05.000Z',
            joinCount: '3',
            previousDisplayNames: [{ displayName: 'Original' }],
            previousDisplayNameSources: {
                friendLog: [{ displayName: 'Friend Log Name' }],
                gameLog: [{ displayName: 'Game Log Name' }]
            }
        });

        const firstRead = readCachedUserStats(key);
        firstRead.previousDisplayNames[0].displayName = 'Mutated';
        firstRead.previousDisplayNameSources!.friendLog[0].displayName =
            'Mutated';
        firstRead.timeSpent = 0;

        expect(readCachedUserStats(key)).toEqual({
            timeSpent: 2000,
            lastSeen: '2026-01-02T03:04:05.000Z',
            friendedAt: '',
            joinCount: 3,
            previousDisplayNames: [{ displayName: 'Original' }],
            previousDisplayNameSources: {
                friendLog: [{ displayName: 'Friend Log Name' }],
                gameLog: [{ displayName: 'Game Log Name' }]
            }
        });
    });

    it('shows the original previous instance list when a dialog-local list is edited then reopened', () => {
        const key = dialogTargetKey('https://api.example.test', 'usr_target');
        cachePreviousInstances(key, [previousInstance('wrld_one:1')]);

        const firstRead = readCachedPreviousInstances(key);
        firstRead.push(previousInstance('wrld_two:2'));

        expect(readCachedPreviousInstances(key)).toEqual([
            previousInstance('wrld_one:1')
        ]);
    });
});
