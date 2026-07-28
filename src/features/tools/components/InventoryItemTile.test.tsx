import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

vi.mock('./MediaAssetTile', () => ({
    MediaAssetTile: ({
        isCurrent,
        currentLabel
    }: {
        isCurrent?: boolean;
        currentLabel?: string;
    }) => (
        <div data-current={String(Boolean(isCurrent))}>
            {currentLabel || ''}
        </div>
    )
}));

import { InventoryItemTile } from './InventoryItemTile';

describe('InventoryItemTile', () => {
    it('forwards the equipped presentation to the shared media tile', () => {
        const html = renderToStaticMarkup(
            <InventoryItemTile
                title="Reference Cube"
                isCurrent
                currentLabel="Equipped"
            />
        );

        expect(html).toContain('data-current="true"');
        expect(html).toContain('Equipped');
    });
});
