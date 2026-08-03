import { describe, expect, it } from 'vitest';

import {
    resolveFavoriteAddType,
    resolveFavoriteEntityLabel,
    resolveRemoteFavoriteGroupLabel
} from './FavoriteActionMenu';

describe('FavoriteActionMenu helpers', () => {
    it('keeps the VRC+ world type selected by the remote group', () => {
        expect(
            resolveFavoriteAddType(
                { type: 'vrcPlusWorld', name: 'worlds4' },
                'world'
            )
        ).toBe('vrcPlusWorld');
    });

    it('uses a fetched friend display name in the remove confirmation', () => {
        expect(
            resolveFavoriteEntityLabel(
                {
                    id: 'usr_friend',
                    displayName: 'Example Friend'
                },
                'usr_friend'
            )
        ).toBe('Example Friend');
    });

    it('falls back to the entity id before profile details are available', () => {
        expect(resolveFavoriteEntityLabel(null, 'usr_friend')).toBe(
            'usr_friend'
        );
    });

    it('shows the remote favorite group display name instead of its key', () => {
        expect(
            resolveRemoteFavoriteGroupLabel(
                {
                    type: 'friend',
                    tags: ['group_0'],
                    $groupKey: 'friend:group_0'
                },
                [
                    {
                        key: 'friend:group_0',
                        name: 'group_0',
                        displayName: 'Best Friends'
                    }
                ]
            )
        ).toBe('Best Friends');
    });

    it('falls back to the remote favorite group key when metadata is unavailable', () => {
        expect(
            resolveRemoteFavoriteGroupLabel(
                {
                    type: 'friend',
                    tags: ['group_0'],
                    $groupKey: 'friend:group_0'
                },
                []
            )
        ).toBe('friend:group_0');
    });
});
