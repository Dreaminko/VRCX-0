import { describe, expect, it } from 'vitest';

import { toolDefinitions } from '@/shared/constants/tools';

import { TELEMETRY_ROUTE_KEYS, TELEMETRY_TOOL_KEYS } from './telemetryContract';

describe('telemetry contract', () => {
    it('contains current route keys without retired route telemetry', () => {
        expect(TELEMETRY_ROUTE_KEYS).toContain('instance_history');
        expect(TELEMETRY_ROUTE_KEYS).toContain('charts_mutual');
        expect(TELEMETRY_ROUTE_KEYS).not.toContain('charts_instance');
        expect(TELEMETRY_ROUTE_KEYS).not.toContain('themes');
        expect(TELEMETRY_ROUTE_KEYS).not.toContain('gallery');
        expect(TELEMETRY_ROUTE_KEYS).not.toContain('inventory');
        expect(TELEMETRY_ROUTE_KEYS).not.toContain('screenshot_metadata');
        expect(TELEMETRY_ROUTE_KEYS).not.toContain('vrchat_log');
    });

    it('contains every current tool key', () => {
        expect([...TELEMETRY_TOOL_KEYS].sort()).toEqual(
            toolDefinitions.map((tool) => tool.key).sort()
        );
    });
});
