import { describe, expect, it } from 'vitest';

import {
    createWorldDialogRequestContext,
    isSameWorldDialogRequestContext
} from './worldDialogRequestContext';

describe('worldDialogRequestContext', () => {
    it('matches the same endpoint, world and open generation', () => {
        const context = createWorldDialogRequestContext({
            endpoint: 'https://api.example.test/api/1',
            openNonce: 4,
            worldId: 'wrld_current'
        });

        expect(
            isSameWorldDialogRequestContext(
                context,
                createWorldDialogRequestContext({
                    endpoint: 'https://api.example.test/api/1',
                    openNonce: 4,
                    worldId: 'wrld_current'
                })
            )
        ).toBe(true);
    });

    it.each([
        ['endpoint', { endpoint: 'https://other' }],
        ['world', { worldId: 'wrld_other' }],
        ['open generation', { openNonce: 5 }]
    ])('rejects a stale %s context', (_label, override) => {
        const active = createWorldDialogRequestContext({
            endpoint: 'https://api.example.test/api/1',
            openNonce: 4,
            worldId: 'wrld_current'
        });
        const candidate = createWorldDialogRequestContext({
            endpoint: 'https://api.example.test/api/1',
            openNonce: 4,
            worldId: 'wrld_current',
            ...override
        });

        expect(isSameWorldDialogRequestContext(active, candidate)).toBe(false);
    });
});
