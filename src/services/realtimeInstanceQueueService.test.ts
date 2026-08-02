import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    success: vi.fn(),
    t: vi.fn((key: string) => key)
}));

vi.mock('sonner', () => ({ toast: { success: mocks.success } }));
vi.mock('@/services/i18nService', () => ({
    default: { t: mocks.t }
}));

import { useLocationHintStore } from '@/state/locationHintStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import { handleRealtimeInstanceQueueProjection } from './realtimeInstanceQueueService';

describe('realtimeInstanceQueueService', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-08-02T00:00:00.000Z'));
        mocks.success.mockReset();
        mocks.t.mockClear();
        useRuntimeStore.getState().resetRuntimeState();
        useLocationHintStore.getState().resetLocationHints();
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it('ignores malformed payloads and payloads without a location', () => {
        handleRealtimeInstanceQueueProjection(null);
        handleRealtimeInstanceQueueProjection({ kind: 'queued' });

        expect(useRuntimeStore.getState().instanceQueue.active).toBe(false);
        expect(mocks.success).not.toHaveBeenCalled();
    });

    it('normalizes queue numbers and reuses the current label', () => {
        useRuntimeStore.getState().setInstanceQueueState({
            active: true,
            instanceLocation: 'wrld_queue:123',
            label: 'Known Queue'
        });

        handleRealtimeInstanceQueueProjection({
            kind: 'queued',
            instanceLocation: ' wrld_queue:123 ',
            position: -3,
            queueSize: 4.6
        });

        expect(useRuntimeStore.getState().instanceQueue).toEqual({
            active: true,
            instanceLocation: 'wrld_queue:123',
            position: 0,
            queueSize: 5,
            label: 'Known Queue',
            updatedAt: '2026-08-02T00:00:00.000Z'
        });
    });

    it('clears ready and left events only when they belong to the active queue', () => {
        useRuntimeStore.getState().setInstanceQueueState({
            active: true,
            instanceLocation: 'wrld_current:123',
            label: 'Current Queue'
        });

        handleRealtimeInstanceQueueProjection({
            kind: 'left',
            instanceLocation: 'wrld_other:456'
        });
        expect(useRuntimeStore.getState().instanceQueue.active).toBe(true);

        handleRealtimeInstanceQueueProjection({
            kind: 'ready',
            instanceLocation: 'wrld_other:456'
        });
        expect(useRuntimeStore.getState().instanceQueue.active).toBe(true);
        expect(mocks.success).toHaveBeenCalledWith(
            'Instance ready to join wrld_other public'
        );

        handleRealtimeInstanceQueueProjection({
            kind: 'left',
            instanceLocation: 'wrld_current:123'
        });
        expect(useRuntimeStore.getState().instanceQueue.active).toBe(false);
    });

    it('uses location hints when a queue label is not already cached', () => {
        useRuntimeStore.getState().setAuthBootstrap({
            currentUserEndpoint: 'https://api.vrchat.cloud/api/1'
        });
        useLocationHintStore.getState().upsertLocationHint({
            endpoint: 'https://api.vrchat.cloud/api/1',
            location: 'wrld_hint:123~group(grp_hint)',
            worldName: 'Hinted World',
            groupName: 'Hinted Group'
        });

        handleRealtimeInstanceQueueProjection({
            kind: 'queued',
            instanceLocation: 'wrld_hint:123~group(grp_hint)',
            position: 2,
            queueSize: 8,
            receivedAt: '2026-08-01T23:59:00.000Z'
        });

        expect(useRuntimeStore.getState().instanceQueue).toMatchObject({
            label: 'Hinted World group(Hinted Group)',
            updatedAt: '2026-08-01T23:59:00.000Z'
        });
    });
});
