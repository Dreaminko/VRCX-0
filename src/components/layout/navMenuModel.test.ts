import { describe, expect, it } from 'vitest';

import { createBaseDefaultNavLayout, routePathByName } from './navMenuModel';

describe('navMenuModel defaults', () => {
    it('places browse history directly after search', () => {
        const layout = createBaseDefaultNavLayout((key: string) => key);
        const searchIndex = layout.findIndex(
            (entry) => entry.type === 'item' && entry.key === 'search'
        );

        expect(routePathByName['browse-history']).toBe('/browse-history');
        expect(layout[searchIndex + 1]).toEqual({
            type: 'item',
            key: 'browse-history'
        });
    });

    it('keeps mutual friends as a top-level default item', () => {
        const layout = createBaseDefaultNavLayout((key: string) => key);

        expect(layout).toContainEqual({ type: 'item', key: 'charts-mutual' });
    });
});
