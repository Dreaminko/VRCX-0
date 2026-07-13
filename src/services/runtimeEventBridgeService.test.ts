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

import { useFriendRosterStore } from '@/state/friendRosterStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';
import { useUserFactsStore } from '@/state/userFactsStore';

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
        updatedAt: '2026-07-09T00:00:00.000Z',
        friendProfileLoad: {
            runId: 0,
            status: 'idle',
            total: 0,
            processed: 0,
            loaded: 0,
            failed: 0,
            startedAt: '',
            finishedAt: null,
            lastError: null
        }
    };
}

describe('runtimeEventBridgeService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        useRuntimeStore.getState().resetRuntimeState();
        useFriendRosterStore.getState().resetRoster();
        useSessionStore.getState().resetSessionState();
        useUserFactsStore.getState().resetUserFacts();
        vi.useRealTimers();
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

    it('batches friend profile projections into one store update', async () => {
        vi.useFakeTimers();
        const handlers = new Map<string, (payload: unknown) => void>();
        mocks.subscribe.mockImplementation((name, handler) => {
            handlers.set(name, handler);
            return Promise.resolve(() => {});
        });
        const cleanup = await bindRuntimeEvents();
        const snapshot = createBackendRuntimeSnapshot();
        useRuntimeStore.getState().setBackendRuntimeSnapshot({
            ...snapshot,
            phase: 'running',
            authStatus: 'authenticated',
            authUserId: 'usr_owner',
            wsStatus: 'connected'
        });
        useRuntimeStore.getState().setAuthBootstrap({
            currentUserId: 'usr_owner'
        });
        useRuntimeStore.getState().setFriendProfileLoadState({
            runId: 1,
            status: 'running'
        });
        useSessionStore.getState().setSessionPhase('ready');
        let rosterUpdates = 0;
        let userFactUpdates = 0;
        const unsubscribeRoster = useFriendRosterStore.subscribe(() => {
            rosterUpdates += 1;
        });
        const unsubscribeUserFacts = useUserFactsStore.subscribe(() => {
            userFactUpdates += 1;
        });

        for (const userId of ['usr_a', 'usr_b']) {
            handlers.get('realtimeUserProjection')?.({
                source: 'friendProfileBulkLoad',
                users: [
                    {
                        id: userId,
                        endpoint: 'api.vrchat.cloud',
                        displayName: userId
                    }
                ]
            });
            handlers.get('realtimeFriendProjection')?.({
                generation: 1,
                baselineRevision: 1,
                source: 'friendProfileBulkLoad',
                patches: [
                    {
                        userId,
                        patch: {
                            id: userId,
                            displayName: userId,
                            state: 'offline'
                        },
                        stateBucket: 'offline',
                        stateBucketAuthority: 'preserve'
                    }
                ],
                removals: [],
                feedEntries: [],
                friendLogChanged: false
            });
        }

        expect(rosterUpdates).toBe(0);
        expect(userFactUpdates).toBe(0);
        await vi.advanceTimersByTimeAsync(9_999);
        expect(rosterUpdates).toBe(0);
        expect(userFactUpdates).toBe(0);
        await vi.advanceTimersByTimeAsync(1);
        expect(rosterUpdates).toBe(1);
        expect(userFactUpdates).toBe(1);
        expect(
            Object.keys(useFriendRosterStore.getState().friendsById)
        ).toEqual(['usr_a', 'usr_b']);

        unsubscribeRoster();
        unsubscribeUserFacts();
        cleanup();
    });

    it.each(['running', 'cancelling'] as const)(
        'applies normal realtime projections immediately while profile loading is %s',
        async (status) => {
            vi.useFakeTimers();
            const handlers = new Map<string, (payload: unknown) => void>();
            mocks.subscribe.mockImplementation((name, handler) => {
                handlers.set(name, handler);
                return Promise.resolve(() => {});
            });
            const cleanup = await bindRuntimeEvents();
            const snapshot = createBackendRuntimeSnapshot();
            useRuntimeStore.getState().setBackendRuntimeSnapshot({
                ...snapshot,
                phase: 'running',
                authStatus: 'authenticated',
                authUserId: 'usr_owner',
                wsStatus: 'connected'
            });
            useRuntimeStore.getState().setAuthBootstrap({
                currentUserId: 'usr_owner'
            });
            useRuntimeStore.getState().setFriendProfileLoadState({
                runId: 1,
                status
            });
            useSessionStore.getState().setSessionPhase('ready');

            handlers.get('realtimeUserProjection')?.({
                source: 'friendProfileBulkLoad',
                users: [
                    {
                        id: 'usr_bulk',
                        endpoint: 'api.vrchat.cloud',
                        displayName: 'Bulk Friend'
                    }
                ]
            });
            handlers.get('realtimeFriendProjection')?.({
                generation: 1,
                baselineRevision: 1,
                source: 'friendProfileBulkLoad',
                patches: [
                    {
                        userId: 'usr_bulk',
                        patch: {
                            id: 'usr_bulk',
                            displayName: 'Bulk Friend'
                        },
                        stateBucket: 'offline',
                        stateBucketAuthority: 'preserve'
                    }
                ],
                removals: [],
                feedEntries: [],
                friendLogChanged: false
            });
            expect(useFriendRosterStore.getState().friendsById).toEqual({});
            expect(useUserFactsStore.getState().usersByKey).toEqual({});

            handlers.get('realtimeUserProjection')?.({
                users: [
                    {
                        id: 'usr_live',
                        endpoint: 'api.vrchat.cloud',
                        displayName: 'Live Friend'
                    }
                ]
            });
            handlers.get('realtimeFriendProjection')?.({
                generation: 1,
                baselineRevision: 1,
                patches: [
                    {
                        userId: 'usr_live',
                        patch: {
                            id: 'usr_live',
                            displayName: 'Live Friend',
                            status: 'offline'
                        },
                        stateBucket: 'online',
                        stateBucketAuthority: 'preserve'
                    }
                ],
                removals: [],
                feedEntries: [],
                friendLogChanged: false
            });

            expect(
                Object.keys(useFriendRosterStore.getState().friendsById)
            ).toEqual(['usr_bulk', 'usr_live']);
            expect(
                Object.values(useUserFactsStore.getState().usersByKey).map(
                    (user) => user.id
                )
            ).toEqual(['usr_bulk', 'usr_live']);
            await vi.advanceTimersByTimeAsync(10_000);
            expect(
                Object.keys(useFriendRosterStore.getState().friendsById)
            ).toEqual(['usr_bulk', 'usr_live']);

            cleanup();
        }
    );

    it('flushes pending friend profile projections when the load becomes terminal', async () => {
        vi.useFakeTimers();
        const handlers = new Map<string, (payload: unknown) => void>();
        mocks.subscribe.mockImplementation((name, handler) => {
            handlers.set(name, handler);
            return Promise.resolve(() => {});
        });
        const cleanup = await bindRuntimeEvents();
        const snapshot = createBackendRuntimeSnapshot();
        useRuntimeStore.getState().setBackendRuntimeSnapshot({
            ...snapshot,
            phase: 'running',
            authStatus: 'authenticated',
            authUserId: 'usr_owner',
            wsStatus: 'connected'
        });
        useRuntimeStore.getState().setAuthBootstrap({
            currentUserId: 'usr_owner'
        });
        useRuntimeStore.getState().setFriendProfileLoadState({
            runId: 100,
            status: 'running'
        });
        useSessionStore.getState().setSessionPhase('ready');

        handlers.get('realtimeUserProjection')?.({
            source: 'friendProfileBulkLoad',
            users: [
                {
                    id: 'usr_terminal',
                    endpoint: 'api.vrchat.cloud',
                    displayName: 'Terminal Friend'
                }
            ]
        });
        handlers.get('realtimeFriendProjection')?.({
            generation: 1,
            baselineRevision: 1,
            source: 'friendProfileBulkLoad',
            patches: [
                {
                    userId: 'usr_terminal',
                    patch: {
                        id: 'usr_terminal',
                        displayName: 'Terminal Friend'
                    },
                    stateBucket: 'offline',
                    stateBucketAuthority: 'preserve'
                }
            ],
            removals: [],
            feedEntries: [],
            friendLogChanged: false
        });
        expect(useFriendRosterStore.getState().friendsById).toEqual({});

        handlers.get('friendProfileLoadStatus')?.({
            runId: 100,
            status: 'cancelled',
            total: 1,
            processed: 0,
            loaded: 0,
            failed: 0,
            startedAt: '2026-07-11T00:00:00.000Z',
            finishedAt: '2026-07-11T00:00:01.000Z',
            lastError: null
        });

        expect(
            Object.keys(useFriendRosterStore.getState().friendsById)
        ).toEqual(['usr_terminal']);
        expect(
            Object.values(useUserFactsStore.getState().usersByKey).map(
                (user) => user.id
            )
        ).toEqual(['usr_terminal']);
        await vi.advanceTimersByTimeAsync(10_000);
        expect(
            Object.keys(useFriendRosterStore.getState().friendsById)
        ).toEqual(['usr_terminal']);

        cleanup();
    });
});
