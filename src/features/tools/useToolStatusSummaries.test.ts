import { describe, expect, it } from 'vitest';

import { countPresenceRules } from './useToolStatusSummaries';

describe('countPresenceRules', () => {
    it('counts missing enabled flags as active and ignores invalid entries', () => {
        expect(
            countPresenceRules([
                { id: 'first', enabled: true },
                { id: 'second', enabled: false },
                { id: 'legacy' },
                null,
                'invalid'
            ])
        ).toEqual({ enabled: 2, total: 3 });
    });

    it('returns zero counts when status loading failed', () => {
        expect(countPresenceRules(null)).toEqual({ enabled: 0, total: 0 });
    });
});
