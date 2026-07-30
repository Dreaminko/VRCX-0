import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useRuntimeStore } from '@/state/runtimeStore';

const mocks = vi.hoisted(() => ({
    appSetGameClientRuntimeState:
        vi.fn<(currentLocation: string) => Promise<void>>(),
    isHostCapabilityAvailable: vi.fn<(key: string) => boolean>()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appSetGameClientRuntimeState: mocks.appSetGameClientRuntimeState
    }
}));

vi.mock('./hostCapabilityService', () => ({
    isHostCapabilityAvailable: mocks.isHostCapabilityAvailable
}));

import { syncRuntimeGameClientState } from './gameClientLifecycle';

describe('GameClient lifecycle routing', () => {
    beforeEach(async () => {
        useRuntimeStore.getState().resetRuntimeState();
        mocks.appSetGameClientRuntimeState.mockReset();
        mocks.isHostCapabilityAvailable.mockReturnValue(false);
        await syncRuntimeGameClientState();
        mocks.isHostCapabilityAvailable.mockReturnValue(true);
    });

    it('syncs an empty location when runtime state has not been sent yet', async () => {
        mocks.appSetGameClientRuntimeState.mockResolvedValue(undefined);

        await syncRuntimeGameClientState();

        expect(mocks.appSetGameClientRuntimeState).toHaveBeenCalledOnce();
        expect(mocks.appSetGameClientRuntimeState).toHaveBeenCalledWith('');
    });

    it('retries clearing the runtime location after a failed sync', async () => {
        const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
        mocks.appSetGameClientRuntimeState.mockResolvedValue(undefined);
        useRuntimeStore.getState().setGameState({
            currentLocation: 'wrld_previous:123'
        });
        await syncRuntimeGameClientState();

        useRuntimeStore.getState().setGameState({ currentLocation: '' });
        mocks.appSetGameClientRuntimeState.mockRejectedValueOnce(
            new Error('transient failure')
        );
        await syncRuntimeGameClientState();
        await syncRuntimeGameClientState();

        expect(mocks.appSetGameClientRuntimeState.mock.calls).toEqual([
            ['wrld_previous:123'],
            [''],
            ['']
        ]);
        warn.mockRestore();
    });
});
