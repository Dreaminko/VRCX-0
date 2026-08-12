import { describe, expect, it } from 'vitest';

import {
    buildFeedRoute,
    readFeedRouteUserIds,
    withFeedRouteUserIds
} from './feedRouteScope';

describe('feedRouteScope', () => {
    it('reads unique scoped users and preserves unrelated parameters', () => {
        const searchParams = new URLSearchParams(
            'user=usr_one&view=table&user=usr_two&user=usr_one'
        );

        expect(readFeedRouteUserIds(searchParams)).toEqual([
            'usr_one',
            'usr_two'
        ]);
        expect(
            withFeedRouteUserIds(searchParams, ['usr_three']).toString()
        ).toBe('view=table&user=usr_three');
    });

    it('builds a Feed path with the selected user', () => {
        expect(buildFeedRoute(['usr_friend'])).toBe(
            '/feed?feedView=table&user=usr_friend'
        );
    });
});
