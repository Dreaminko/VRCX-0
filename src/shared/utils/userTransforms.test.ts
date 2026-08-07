import { describe, expect, it } from 'vitest';

import {
    computeTrustLevel,
    computeUserPlatform,
    diffObjectProps,
    sanitizeUserJson
} from './userTransforms';

describe('computeTrustLevel', () => {
    it('ranks VRChat trust ranks from Visitor to Trusted User, driving both the label and the sort order used in friend/player lists', () => {
        expect(computeTrustLevel([], '')).toMatchObject({
            trustLevel: 'Visitor',
            trustColorKey: 'untrusted',
            trustSortNum: 1
        });
        expect(computeTrustLevel(['system_trust_veteran'], '')).toMatchObject({
            trustLevel: 'Trusted User',
            trustColorKey: 'veteran',
            trustSortNum: 5
        });
        expect(computeTrustLevel(['system_trust_known'], '')).toMatchObject({
            trustLevel: 'User',
            trustSortNum: 3
        });
    });

    it('flags a moderator/admin with a VIP color that overrides their trust-rank color, so staff always stand out in the list', () => {
        const result = computeTrustLevel(['system_trust_veteran'], 'moderator');
        expect(result.isModerator).toBe(true);
        expect(result.trustColorKey).toBe('vip');
    });

    it('flags a known troll with a distinct color even though their underlying trust rank is unaffected, warning the user before they interact', () => {
        const result = computeTrustLevel(
            ['system_trust_known', 'system_troll'],
            ''
        );
        expect(result.isTroll).toBe(true);
        expect(result.trustColorKey).toBe('troll');
        expect(result.trustLevel).toBe('User');
    });

    it('treats a confirmed troll tag as taking priority over a merely probable-troll tag', () => {
        const result = computeTrustLevel(
            ['system_troll', 'system_probable_troll'],
            ''
        );
        expect(result.isTroll).toBe(true);
        expect(result.isProbableTroll).toBe(false);
    });
});

describe('computeUserPlatform', () => {
    it('reports the platform the user is currently active on', () => {
        expect(computeUserPlatform('android', 'standalonewindows')).toBe(
            'android'
        );
    });

    it('falls back to the last known platform when the user is offline, so the friend list still shows where they last played from', () => {
        expect(computeUserPlatform('offline', 'android')).toBe('android');
    });

    it('falls back to the last known platform for a web-session presence, since "web" is not a real client platform worth displaying', () => {
        expect(computeUserPlatform('web', 'standalonewindows')).toBe(
            'standalonewindows'
        );
    });

    it('shows nothing rather than a stale guess when there is no last-known platform either', () => {
        expect(computeUserPlatform('offline', undefined)).toBe('');
    });
});

describe('diffObjectProps', () => {
    it('detects a changed scalar field, reporting both the old and new value for a change-log/notification', () => {
        const result = diffObjectProps(
            { displayName: 'Old Name' },
            { displayName: 'New Name' },
            () => true
        );
        expect(result.hasPropChanged).toBe(true);
        expect(result.changedProps.displayName).toEqual([
            'New Name',
            'Old Name'
        ]);
    });

    it('does not report a field as changed when the incoming value is identical', () => {
        const result = diffObjectProps(
            { displayName: 'Same' },
            { displayName: 'Same' },
            () => true
        );
        expect(result.hasPropChanged).toBe(false);
        expect(result.changedProps).toEqual({});
    });

    it('uses the caller-supplied array comparator for array fields, so tag-list reordering does not falsely count as a change', () => {
        const arraysMatchFn = (a: unknown[], b: unknown[]) =>
            JSON.stringify([...a].sort()) === JSON.stringify([...b].sort());

        const unchanged = diffObjectProps(
            { tags: ['a', 'b'] },
            { tags: ['b', 'a'] },
            arraysMatchFn
        );
        expect(unchanged.hasPropChanged).toBe(false);

        const changed = diffObjectProps(
            { tags: ['a', 'b'] },
            { tags: ['a', 'c'] },
            arraysMatchFn
        );
        expect(changed.hasPropChanged).toBe(true);
    });

    it('ignores a field that only exists on one side, since that is a schema difference rather than a live user update', () => {
        const result = diffObjectProps(
            { displayName: 'Name' },
            { displayName: 'Name', newField: 'value' },
            () => true
        );
        expect(result.hasPropChanged).toBe(false);
    });
});

describe('sanitizeUserJson', () => {
    it('clears the avatar image URLs when they point at the fallback "robot" placeholder, so the UI shows its own empty state instead of a placeholder image', () => {
        const robotUrl = 'https://api.vrchat.cloud/file/robot.png';
        const result = sanitizeUserJson(
            {
                currentAvatarImageUrl: robotUrl,
                currentAvatarThumbnailImageUrl: robotUrl
            },
            robotUrl
        );
        expect(result.currentAvatarImageUrl).toBeUndefined();
        expect(result.currentAvatarThumbnailImageUrl).toBeUndefined();
    });

    it('leaves a real avatar image URL untouched', () => {
        const result = sanitizeUserJson(
            { currentAvatarImageUrl: 'https://files.vrchat.cloud/real.png' },
            'https://api.vrchat.cloud/file/robot.png'
        );
        expect(result.currentAvatarImageUrl).toBe(
            'https://files.vrchat.cloud/real.png'
        );
    });
});
