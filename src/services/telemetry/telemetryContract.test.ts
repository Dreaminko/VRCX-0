import { describe, expect, it } from 'vitest';

import { toolDefinitions } from '@/shared/constants/tools';

import { TELEMETRY_ROUTE_KEYS, TELEMETRY_TOOL_KEYS } from './telemetryContract';

describe('telemetry contract', () => {
    it('contains current route keys', () => {
        expect(TELEMETRY_ROUTE_KEYS).toContain('instance_history');
        expect(TELEMETRY_ROUTE_KEYS).toContain('charts_mutual');
    });

    it('contains every current tool key', () => {
        expect([...TELEMETRY_TOOL_KEYS].sort()).toEqual(
            toolDefinitions.map((tool) => tool.key).sort()
        );
    });
});
