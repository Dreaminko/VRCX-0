import { beforeEach, describe, expect, it, vi } from 'vitest';

const { navigate, recordToolOpen } = vi.hoisted(() => ({
    navigate: vi.fn(),
    recordToolOpen: vi.fn()
}));

vi.mock('sonner', () => ({
    toast: {
        error: vi.fn(),
        success: vi.fn()
    }
}));
vi.mock('@/platform/tauri/bindings', () => ({
    commands: {}
}));
vi.mock('@/services/hostCapabilityService', () => ({
    getHostCapabilityUnavailableReason: vi.fn(() => 'Unavailable'),
    isHostCapabilityAvailable: vi.fn(() => false),
    isHostCapabilitySupported: vi.fn(() => false)
}));
vi.mock('@/services/i18nService', () => ({
    default: { t: vi.fn((key: string) => key) }
}));
vi.mock('@/services/telemetry/telemetryToolUsage', () => ({
    recordToolOpen
}));

import { triggerToolByKey } from './toolActionService';

describe('tool action telemetry', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('records one open when dispatching an available tool', async () => {
        await triggerToolByKey('inventory', {
            navigate,
            t: (key) => key
        });

        expect(recordToolOpen).toHaveBeenCalledOnce();
        expect(recordToolOpen).toHaveBeenCalledWith('inventory');
        expect(navigate).toHaveBeenCalledWith('/tools/inventory');
    });

    it('does not record unknown or unavailable tools', async () => {
        await triggerToolByKey('unknown-tool', {
            navigate,
            t: (key) => key
        });
        await triggerToolByKey('vrc-photos', {
            navigate,
            t: (key) => key
        });

        expect(recordToolOpen).not.toHaveBeenCalled();
        expect(navigate).not.toHaveBeenCalled();
    });
});
