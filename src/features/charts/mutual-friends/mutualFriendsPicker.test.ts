import { describe, expect, it } from 'vitest';

import type { FriendRecord } from '@/domain/friends/friendRosterTypes';

import {
    buildMutualFriendExcludePickerOptions,
    buildMutualFriendPickerOption,
    filterMutualFriendPickerOptions,
    mutualFriendPickerOptionMatches,
    truncateMutualFriendLabel
} from './mutualFriendsPicker';
import { MUTUAL_GRAPH_EMPTY_USER_ID } from './mutualFriendsSettings';
import type { MutualFriendPickerOption } from './mutualFriendsTypes';

function friend(patch: Partial<FriendRecord> = {}): FriendRecord {
    return {
        id: 'usr_1',
        displayName: 'Friend',
        tags: [],
        state: 'offline',
        stateBucket: 'offline',
        $trustLevel: 'Visitor',
        $friendNumber: 0,
        $trustClass: 'x-tag-untrusted',
        $trustSortNum: 0,
        $isModerator: false,
        $isTroll: false,
        $isProbableTroll: false,
        $platform: '',
        ...patch
    };
}

function option(value: string, label: string): MutualFriendPickerOption {
    return {
        value,
        label,
        displayLabel: label,
        search: `${label} ${value}`,
        user: null
    };
}

describe('mutualFriendsPicker', () => {
    it('searches picker options by the text users can see or identify', () => {
        const built = buildMutualFriendPickerOption(
            ' usr_ava ',
            {
                usr_ava: friend({
                    id: 'usr_ava',
                    displayName: 'Ava Star',
                    username: 'ava_user'
                })
            },
            '',
            5
        );

        expect(mutualFriendPickerOptionMatches(built, 'ava usr_ava')).toBe(
            true
        );
        expect(mutualFriendPickerOptionMatches(built, 'missing')).toBe(false);
        expect(
            filterMutualFriendPickerOptions(
                [built, option('usr_ben', 'Ben')].filter(
                    (item): item is MutualFriendPickerOption => Boolean(item)
                ),
                'usr',
                1
            )
        ).toHaveLength(1);
    });

    it('keeps selected exclude-picker options at the top before limiting results', () => {
        const options = filterMutualFriendPickerOptions(
            [
                option('usr_a', 'Ava'),
                option('usr_b', 'Ben'),
                option('usr_c', 'Cyd')
            ],
            '',
            2,
            new Set(['usr_c'])
        );

        expect(options.map((item) => item.value)).toEqual(['usr_c', 'usr_a']);
    });

    it('builds hidden-friend picker choices from all cached graph ids without duplicates or self', () => {
        const options = buildMutualFriendExcludePickerOptions(
            new Map([
                ['usr_self', ['usr_a', 'usr_b']],
                ['usr_a', ['usr_self', 'usr_b', MUTUAL_GRAPH_EMPTY_USER_ID]]
            ]),
            {
                usr_a: friend({ id: 'usr_a', displayName: 'Ava' }),
                usr_b: friend({ id: 'usr_b', displayName: 'Ben' })
            },
            'usr_self'
        );

        expect(options.map((item) => item.value)).toEqual(['usr_a', 'usr_b']);
        expect(options.map((item) => item.label)).toEqual(['Ava', 'Ben']);
    });

    it('keeps long graph labels compact for node rendering', () => {
        expect(truncateMutualFriendLabel('Short name', 20)).toBe('Short name');
        expect(truncateMutualFriendLabel('Very long display name', 10)).toBe(
            'Very long…'
        );
    });
});
