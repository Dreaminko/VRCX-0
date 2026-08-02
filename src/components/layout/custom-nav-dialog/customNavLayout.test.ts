import { describe, expect, it } from 'vitest';

import {
    buildHiddenPlacementMap,
    buildVisibleNodes,
    cloneLayout,
    getFolderDropId,
    insertKeyIntoLayout,
    removeKeyFromLayout,
    resolveDragNode
} from './customNavLayout';

describe('customNavLayout', () => {
    it('clones valid entries while normalizing icons and dropping invalid data', () => {
        expect(
            cloneLayout([
                null,
                { type: 'unknown', key: 'ignored' },
                { type: 'item', key: 'feed', icon: 'RssIcon' },
                {
                    type: 'folder',
                    id: 'social',
                    name: 'Social',
                    icon: 'invalid',
                    items: [
                        'friends',
                        { key: 'groups', icon: 'Users' },
                        { icon: 'Heart' }
                    ]
                }
            ])
        ).toEqual([
            { type: 'item', key: 'feed', icon: 'lucide:Rss' },
            {
                type: 'folder',
                id: 'social',
                name: 'Social',
                nameKey: null,
                icon: 'lucide:Folder',
                items: ['friends', { key: 'groups', icon: 'lucide:Users' }]
            }
        ]);
    });

    it('round-trips hidden root and folder items with their positions and icons', () => {
        const layout = [
            { type: 'item' as const, key: 'home' },
            {
                type: 'folder' as const,
                id: 'social',
                name: 'Social',
                items: ['friends', { key: 'groups', icon: 'lucide:Users' }]
            },
            { type: 'item' as const, key: 'tools', icon: 'lucide:Wrench' }
        ];

        const withoutGroups = removeKeyFromLayout(layout, 'groups');
        expect(withoutGroups.placement).toEqual({
            parentId: 'social',
            index: 1,
            icon: 'lucide:Users'
        });
        expect(
            insertKeyIntoLayout(
                withoutGroups.layout,
                'groups',
                withoutGroups.placement
            )
        ).toEqual(cloneLayout(layout));

        const withoutTools = removeKeyFromLayout(layout, 'tools');
        expect(
            insertKeyIntoLayout(
                withoutTools.layout,
                'tools',
                withoutTools.placement
            )
        ).toEqual(cloneLayout(layout));
    });

    it('falls back to the top level when a saved folder no longer exists', () => {
        expect(
            insertKeyIntoLayout([], 'friends', {
                parentId: 'removed-folder',
                index: 4,
                icon: 'lucide:Heart'
            })
        ).toEqual([{ type: 'item', key: 'friends', icon: 'lucide:Heart' }]);
    });

    it('builds stable hidden placements and drag nodes', () => {
        const layout = [
            { type: 'item' as const, key: 'home' },
            {
                type: 'folder' as const,
                id: 'social',
                items: [{ key: 'friends', icon: 'lucide:Heart' }]
            }
        ];
        const placements = buildHiddenPlacementMap(layout, ['home', 'friends']);

        expect(placements.get('home')).toEqual({
            parentId: null,
            index: 0,
            icon: undefined
        });
        expect(placements.get('friends')).toEqual({
            parentId: 'social',
            index: 0,
            icon: 'lucide:Heart'
        });

        const nodes = buildVisibleNodes(layout);
        expect(resolveDragNode('item:friends', nodes)).toMatchObject({
            type: 'item',
            id: 'friends',
            parentId: 'social'
        });
        expect(resolveDragNode(getFolderDropId('social'), nodes)).toEqual({
            type: 'folder-drop',
            id: 'social',
            parentId: null,
            sortableId: 'folder-drop:social'
        });
    });
});
