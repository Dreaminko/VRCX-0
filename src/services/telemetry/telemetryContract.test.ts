import { describe, expect, it } from 'vitest';

import { TELEMETRY_ROUTE_KEYS } from './telemetryContract';

describe('telemetry contract', () => {
    it('contains current route keys without retired route telemetry', () => {
        expect(TELEMETRY_ROUTE_KEYS).toContain('instance_history');
        expect(TELEMETRY_ROUTE_KEYS).toContain('charts_mutual');
        expect(TELEMETRY_ROUTE_KEYS).not.toContain('charts_instance');
        expect(TELEMETRY_ROUTE_KEYS).not.toContain('themes');
    });
});
