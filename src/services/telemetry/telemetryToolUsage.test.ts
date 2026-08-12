import { afterEach, describe, expect, it, vi } from 'vitest';

import type { TelemetryClientEvent } from '@/platform/tauri/bindings';

afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
});

function mockTelemetryCommand() {
    const appTelemetryRecordEvent = vi.fn((event: TelemetryClientEvent) => {
        void event;
        return Promise.resolve(null);
    });
    vi.doMock('@/platform/tauri/bindings', () => ({
        commands: { appTelemetryRecordEvent }
    }));
    return { appTelemetryRecordEvent };
}

describe('tool usage telemetry', () => {
    it('records one canonical tool-open event per accepted activation', async () => {
        const { appTelemetryRecordEvent } = mockTelemetryCommand();
        const mod = await import('./telemetryToolUsage');

        mod.recordToolOpen('profile-backup');
        mod.recordToolOpen('unknown-tool');

        expect(appTelemetryRecordEvent).toHaveBeenCalledOnce();
        expect(appTelemetryRecordEvent).toHaveBeenCalledWith({
            type: 'toolOpen',
            tool: 'profile-backup'
        });
    });
});
