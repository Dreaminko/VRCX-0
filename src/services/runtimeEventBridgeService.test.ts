import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { BackendRuntimeSnapshot } from '@/platform/tauri/bindings';

const mocks = vi.hoisted(() => ({
    subscribe:
        vi.fn<
            (
                name: string,
                handler: (payload: unknown) => void
            ) => Promise<() => void>
        >(),
    applyRuntimeGameLogProjection: vi.fn(),
    recordRuntimeGameClientEvent: vi.fn(),
    handleGameRunningUpdate: vi.fn<() => Promise<void>>(),
    isHostCapabilityAvailable: vi.fn<(name: string) => boolean>(),
    refreshHostCapabilities: vi.fn(),
    handleIpcEvent: vi.fn<() => Promise<void>>(),
    pushSharedFeedNotification: vi.fn<() => Promise<void>>(),
    showSQLiteErrorDialog: vi.fn<() => Promise<void>>(),
    handleBrowserFocus: vi.fn<() => Promise<void>>(),
    getBackendRuntimeSnapshot:
        vi.fn<
            () => Promise<
                import('@/platform/tauri/bindings').BackendRuntimeSnapshot
            >
        >(),
    runtimeGroupInstancesRefresh: vi.fn<() => Promise<null>>(),
    appCheckGameRunning: vi.fn<() => Promise<null>>(),
    bindDeepLinkEvents: vi.fn<() => Promise<() => void>>(),
    drainPendingDeepLinks: vi.fn<() => Promise<void>>(),
    deepLinkUnsubscribe: vi.fn(),
    resumeFrontendSessionFromBackendRuntime:
        vi.fn<(snapshot: unknown) => Promise<boolean>>()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appCheckGameRunning: mocks.appCheckGameRunning,
        appGetBackendRuntimeSnapshot: mocks.getBackendRuntimeSnapshot,
        appRuntimeGroupInstancesRefresh: mocks.runtimeGroupInstancesRefresh
    }
}));

vi.mock('@/platform/tauri/client', () => ({
    tauriClient: {
        events: {
            subscribe: mocks.subscribe
        }
    }
}));

vi.mock('./gameLogIngestService', () => ({
    applyRuntimeGameLogProjection: mocks.applyRuntimeGameLogProjection
}));

vi.mock('./gameClientLifecycle', () => ({
    recordRuntimeGameClientEvent: mocks.recordRuntimeGameClientEvent
}));

vi.mock('./gameStateService', () => ({
    handleGameRunningUpdate: mocks.handleGameRunningUpdate
}));

vi.mock('./hostCapabilityService', () => ({
    isHostCapabilityAvailable: mocks.isHostCapabilityAvailable
}));

vi.mock('./ipcEventService', () => ({
    handleIpcEvent: mocks.handleIpcEvent
}));

vi.mock('./sharedFeedNotificationService', () => ({
    pushSharedFeedNotification: mocks.pushSharedFeedNotification
}));

vi.mock('./sqliteErrorDialogService', () => ({
    showSQLiteErrorDialog: mocks.showSQLiteErrorDialog
}));

vi.mock('./vrcStatusService', () => ({
    handleBrowserFocus: mocks.handleBrowserFocus
}));

vi.mock('./backendRuntimeSessionResumeService', () => ({
    resumeFrontendSessionFromBackendRuntime:
        mocks.resumeFrontendSessionFromBackendRuntime
}));

vi.mock('./deepLinkService', () => ({
    bindDeepLinkEvents: mocks.bindDeepLinkEvents,
    drainPendingDeepLinks: mocks.drainPendingDeepLinks
}));

import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';

import { bindRuntimeEvents } from './runtimeEventBridgeService';

function createBackendRuntimeSnapshot(): BackendRuntimeSnapshot {
    return {
        mode: 'foreground',
        phase: 'idle',
        authStatus: 'none',
        authUserId: '',
        authDisplayName: '',
        wsStatus: 'idle',
        gameLogStatus: 'idle',
        processStatus: 'idle',
        wsMessageCounts: {},
        wsPersistedCount: 0,
        gameLogPersistedCount: 0,
        lastError: null,
        updatedAt: '2026-07-09T00:00:00.000Z'
    };
}

describe('runtimeEventBridgeService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        useRuntimeStore.getState().resetRuntimeState();
        useSessionStore.getState().resetSessionState();
        mocks.isHostCapabilityAvailable.mockReturnValue(false);
        mocks.subscribe.mockResolvedValue(() => {});
        mocks.getBackendRuntimeSnapshot.mockResolvedValue(
            createBackendRuntimeSnapshot()
        );
        mocks.runtimeGroupInstancesRefresh.mockResolvedValue(null);
        mocks.appCheckGameRunning.mockResolvedValue(null);
        mocks.bindDeepLinkEvents.mockResolvedValue(mocks.deepLinkUnsubscribe);
        mocks.drainPendingDeepLinks.mockResolvedValue(undefined);
        mocks.resumeFrontendSessionFromBackendRuntime.mockResolvedValue(false);
    });

    it('drains pending deep links after backend runtime snapshot hydration', async () => {
        const calls: string[] = [];
        mocks.bindDeepLinkEvents.mockImplementation(async () => {
            calls.push('bind-deep-link-events');
            return mocks.deepLinkUnsubscribe;
        });
        mocks.getBackendRuntimeSnapshot.mockImplementation(async () => {
            calls.push('get-backend-snapshot');
            return createBackendRuntimeSnapshot();
        });
        mocks.resumeFrontendSessionFromBackendRuntime.mockImplementation(
            async () => {
                calls.push('hydrate-backend-snapshot');
                return false;
            }
        );
        mocks.drainPendingDeepLinks.mockImplementation(async () => {
            calls.push('drain-deep-links');
        });

        await bindRuntimeEvents();

        expect(calls).toEqual([
            'get-backend-snapshot',
            'hydrate-backend-snapshot',
            'bind-deep-link-events',
            'drain-deep-links'
        ]);
        expect(mocks.drainPendingDeepLinks).toHaveBeenCalledTimes(1);
    });

    it('records GameLog persistence fallback as telemetry without frontend ingest', async () => {
        const handlers = new Map<string, (payload: unknown) => void>();
        const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
        mocks.subscribe.mockImplementation((name, handler) => {
            handlers.set(name, handler);
            return Promise.resolve(() => {});
        });

        await bindRuntimeEvents();

        handlers.get('gameLogPersistenceFallback')?.({
            error: 'database is locked',
            batch: {
                video_plays: [
                    {
                        created_at: '2026-05-15T00:00:00.000Z',
                        video_url: 'https://video.example.test'
                    }
                ]
            },
            rawRows: [
                [
                    'runtime-game-log',
                    '2026-05-15T00:00:00.000Z',
                    'video-play',
                    'https://video.example.test',
                    ''
                ]
            ]
        });

        expect(mocks.showSQLiteErrorDialog).not.toHaveBeenCalled();
        expect(
            useRuntimeStore.getState().runtimeEvents.gameLogPersistenceFallback
                .count
        ).toBe(1);
        expect(warn).toHaveBeenCalledWith(
            'Backend GameLog persistence failed:',
            'database is locked'
        );

        warn.mockRestore();
    });

    it('records runtime-persisted GameLog mirrors without frontend ingest', async () => {
        const handlers = new Map<string, (payload: unknown) => void>();
        mocks.subscribe.mockImplementation((name, handler) => {
            handlers.set(name, handler);
            return Promise.resolve(() => {});
        });
        await bindRuntimeEvents();

        const payload = {
            runtimePersisted: true,
            raw: [
                'runtime-game-log',
                '2026-05-15T00:00:00.000Z',
                'location',
                'wrld_test:1',
                'Test World'
            ]
        };
        handlers.get('addGameLogEvent')?.(payload);

        expect(
            useRuntimeStore.getState().runtimeEvents.addGameLogEvent.count
        ).toBe(1);
    });

    it('applies runtime GameLog projection when runtime ingest is active', async () => {
        const handlers = new Map<string, (payload: unknown) => void>();
        mocks.subscribe.mockImplementation((name, handler) => {
            handlers.set(name, handler);
            return Promise.resolve(() => {});
        });
        mocks.isHostCapabilityAvailable.mockImplementation(
            (name) => name === 'runtimeGameLogIngest'
        );

        await bindRuntimeEvents();

        const payload = {
            currentLocation: 'wrld_test:1',
            currentWorldName: 'Test World',
            currentLocationPlayers: []
        };
        handlers.get('gameLogProjection')?.(payload);

        expect(mocks.applyRuntimeGameLogProjection).toHaveBeenCalledWith(
            payload
        );
        expect(
            useRuntimeStore.getState().runtimeEvents.gameLogProjection.count
        ).toBe(1);
    });
});
