import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    runtimeState: {
        startup: { services: { status: 'idle' } },
        setStartupTask: vi.fn()
    },
    sessionState: {
        isFavoritesLoaded: false,
        isFriendsLoaded: false,
        transportStatus: 'disconnected'
    }
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: { getState: () => mocks.runtimeState }
}));

vi.mock('@/state/sessionStore', () => ({
    useSessionStore: { getState: () => mocks.sessionState }
}));

import {
    getPendingStartupServices,
    syncStartupServicesTask
} from './startupServicesStatus';

describe('startupServicesStatus', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.runtimeState.startup.services.status = 'idle';
        mocks.sessionState.isFriendsLoaded = false;
        mocks.sessionState.transportStatus = 'disconnected';
        mocks.sessionState.isFavoritesLoaded = false;
    });

    it('reports every startup dependency in owner-path order', () => {
        expect(getPendingStartupServices()).toEqual([
            'friend roster baseline',
            'realtime transport',
            'favorites hydration'
        ]);

        expect(syncStartupServicesTask(['Bootstrap ready.'])).toEqual({
            completed: false,
            pending: [
                'friend roster baseline',
                'realtime transport',
                'favorites hydration'
            ],
            detail: 'Bootstrap ready. Pending: friend roster baseline, realtime transport, and favorites hydration.'
        });
        expect(mocks.runtimeState.setStartupTask).toHaveBeenCalledWith(
            'services',
            'pending',
            'Bootstrap ready. Pending: friend roster baseline, realtime transport, and favorites hydration.'
        );
    });

    it('formats one and two pending dependencies naturally', () => {
        mocks.sessionState.isFriendsLoaded = true;

        expect(syncStartupServicesTask().detail).toBe(
            'Pending: realtime transport and favorites hydration.'
        );

        mocks.sessionState.transportStatus = 'pipeline-connected';
        expect(syncStartupServicesTask().detail).toBe(
            'Pending: favorites hydration.'
        );
    });

    it('marks services completed only after all three owners are ready', () => {
        mocks.sessionState.isFriendsLoaded = true;
        mocks.sessionState.transportStatus = 'pipeline-connected';
        mocks.sessionState.isFavoritesLoaded = true;

        const result = syncStartupServicesTask();

        expect(result.completed).toBe(true);
        expect(result.pending).toEqual([]);
        expect(result.detail).toBe(
            'Friend roster baseline, realtime transport, and favorites hydration are active.'
        );
        expect(mocks.runtimeState.setStartupTask).toHaveBeenCalledWith(
            'services',
            'completed',
            result.detail
        );
    });

    it('does not overwrite an existing startup error', () => {
        mocks.runtimeState.startup.services.status = 'error';
        mocks.sessionState.isFriendsLoaded = true;
        mocks.sessionState.transportStatus = 'pipeline-connected';
        mocks.sessionState.isFavoritesLoaded = true;

        syncStartupServicesTask();

        expect(mocks.runtimeState.setStartupTask).toHaveBeenCalledWith(
            'services',
            'error',
            expect.any(String)
        );
    });
});
