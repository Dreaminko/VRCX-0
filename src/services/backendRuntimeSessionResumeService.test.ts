import { beforeEach, describe, expect, it, vi } from 'vitest';

import type {
    BackendRuntimeFrontendSessionSnapshot,
    BackendRuntimeSnapshot,
    RuntimeAuthScopeSnapshot
} from '@/platform/tauri/bindings';

const mocks = vi.hoisted(() => ({
    appRuntimeAuthScopeGet: vi.fn<() => Promise<RuntimeAuthScopeSnapshot>>(),
    appGetBackendRuntimeFrontendSessionSnapshot:
        vi.fn<
            (
                includeCurrentUserSnapshot: boolean
            ) => Promise<BackendRuntimeFrontendSessionSnapshot | null>
        >(),
    recordCurrentUserSnapshot: vi.fn(),
    bootstrapAuthenticatedSession: vi.fn(),
    loadVrchatConfigSnapshot: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appRuntimeAuthScopeGet: mocks.appRuntimeAuthScopeGet,
        appGetBackendRuntimeFrontendSessionSnapshot:
            mocks.appGetBackendRuntimeFrontendSessionSnapshot
    }
}));

vi.mock('./domainIngestionService', () => ({
    recordCurrentUserSnapshot: mocks.recordCurrentUserSnapshot
}));

vi.mock('./sessionBootstrapService', () => ({
    bootstrapAuthenticatedSession: mocks.bootstrapAuthenticatedSession
}));

vi.mock('./vrchatConfigService', () => ({
    loadVrchatConfigSnapshot: mocks.loadVrchatConfigSnapshot
}));

import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';

import { AuthAttemptSupersededError } from './authAttempt';
import { resumeFrontendSessionFromBackendRuntime } from './backendRuntimeSessionResumeService';

const USER_ID = 'usr_owner';
const ENDPOINT = 'https://api.vrchat.cloud/api/1';
const WEBSOCKET = 'wss://pipeline.vrchat.cloud';

function backendSnapshot(
    patch: Partial<BackendRuntimeSnapshot> = {}
): BackendRuntimeSnapshot {
    return {
        mode: 'foreground',
        phase: 'running',
        authStatus: 'authenticated',
        authUserId: USER_ID,
        authDisplayName: 'Backend User',
        wsStatus: 'connected',
        gameLogStatus: 'running',
        processStatus: 'unknown',
        gameLogPersistedCount: 0,
        lastError: null,
        updatedAt: '2026-08-04T00:00:00.000Z',
        friendProfileLoad: {
            runId: 1,
            status: 'idle',
            total: 0,
            processed: 0,
            loaded: 0,
            failed: 0,
            startedAt: '',
            finishedAt: null
        },
        ...patch
    };
}

function authScope(
    patch: Partial<RuntimeAuthScopeSnapshot> = {}
): RuntimeAuthScopeSnapshot {
    return {
        currentUserId: USER_ID,
        endpoint: ENDPOINT,
        generation: 3,
        active: true,
        ...patch
    };
}

function frontendSession(
    patch: Partial<BackendRuntimeFrontendSessionSnapshot> = {}
): BackendRuntimeFrontendSessionSnapshot {
    return {
        authenticated: true,
        userId: USER_ID,
        displayName: 'Frontend User',
        endpoint: ENDPOINT,
        websocket: WEBSOCKET,
        currentUserSnapshot: {
            id: USER_ID,
            displayName: 'Frontend User',
            username: 'frontend_user'
        },
        ...patch
    };
}

function deferred<T>() {
    let resolve: (value: T) => void = () => {
        throw new Error('Deferred promise was not initialized.');
    };
    const promise = new Promise<T>((next) => {
        resolve = next;
    });
    return { promise, resolve };
}

function setCurrentBackendRuntime(snapshot = backendSnapshot()): void {
    useRuntimeStore.getState().setBackendRuntimeSnapshot(snapshot);
}

describe('backendRuntimeSessionResumeService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        useRuntimeStore.getState().resetRuntimeState();
        useSessionStore.getState().resetSessionState();
        mocks.appRuntimeAuthScopeGet.mockResolvedValue(authScope());
        mocks.appGetBackendRuntimeFrontendSessionSnapshot.mockReset();
        mocks.appGetBackendRuntimeFrontendSessionSnapshot.mockImplementation(
            async (includeCurrentUserSnapshot) =>
                frontendSession({
                    currentUserSnapshot: includeCurrentUserSnapshot
                        ? frontendSession().currentUserSnapshot
                        : null
                })
        );
        mocks.bootstrapAuthenticatedSession.mockResolvedValue(undefined);
        mocks.loadVrchatConfigSnapshot.mockResolvedValue({});
        setCurrentBackendRuntime();
    });

    it('rejects a snapshot whose user is no longer the current backend owner', async () => {
        setCurrentBackendRuntime(backendSnapshot({ authUserId: 'usr_other' }));

        await expect(
            resumeFrontendSessionFromBackendRuntime(backendSnapshot())
        ).resolves.toBe(false);

        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
        expect(mocks.bootstrapAuthenticatedSession).not.toHaveBeenCalled();
    });

    it('rejects an auth scope owned by a different user', async () => {
        mocks.appRuntimeAuthScopeGet.mockResolvedValueOnce(
            authScope({ currentUserId: 'usr_other' })
        );

        await expect(
            resumeFrontendSessionFromBackendRuntime(backendSnapshot())
        ).resolves.toBe(false);

        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
        expect(mocks.bootstrapAuthenticatedSession).not.toHaveBeenCalled();
    });

    it('rejects a frontend session snapshot owned by a different user', async () => {
        mocks.appGetBackendRuntimeFrontendSessionSnapshot.mockResolvedValueOnce(
            frontendSession({ userId: 'usr_other' })
        );

        await expect(
            resumeFrontendSessionFromBackendRuntime(backendSnapshot())
        ).resolves.toBe(false);

        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
        expect(mocks.bootstrapAuthenticatedSession).not.toHaveBeenCalled();
    });

    it('discards query results when the frontend starts authenticating while they are in flight', async () => {
        const scopeRequest = deferred<RuntimeAuthScopeSnapshot>();
        mocks.appRuntimeAuthScopeGet.mockReturnValueOnce(scopeRequest.promise);

        const resume =
            resumeFrontendSessionFromBackendRuntime(backendSnapshot());
        useSessionStore.getState().setSessionPhase('authenticating');
        scopeRequest.resolve(authScope());

        await expect(resume).resolves.toBe(false);
        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
        expect(mocks.bootstrapAuthenticatedSession).not.toHaveBeenCalled();
    });

    it('keeps a ready frontend session unchanged when its connection already matches', async () => {
        useRuntimeStore.getState().setAuthBootstrap({
            currentUserId: USER_ID,
            currentUserDisplayName: 'Current User',
            currentUserEndpoint: ENDPOINT,
            currentUserWebsocket: WEBSOCKET,
            currentUserSnapshot: {
                id: USER_ID,
                displayName: 'Current User'
            }
        });
        useSessionStore.getState().setSessionState({
            isLoggedIn: true,
            sessionPhase: 'ready'
        });

        await expect(
            resumeFrontendSessionFromBackendRuntime(backendSnapshot())
        ).resolves.toBe(false);

        expect(useRuntimeStore.getState().auth.currentUserDisplayName).toBe(
            'Current User'
        );
        expect(
            mocks.appGetBackendRuntimeFrontendSessionSnapshot
        ).toHaveBeenCalledWith(false);
        expect(mocks.recordCurrentUserSnapshot).not.toHaveBeenCalled();
        expect(mocks.bootstrapAuthenticatedSession).not.toHaveBeenCalled();
    });

    it('updates only the ready session mirror when its backend connection changes', async () => {
        const nextEndpoint = 'https://api.example.test/api/1';
        const nextWebsocket = 'wss://pipeline.example.test';
        const nextFrontendSession = frontendSession({
            endpoint: nextEndpoint,
            websocket: nextWebsocket
        });
        mocks.appGetBackendRuntimeFrontendSessionSnapshot
            .mockResolvedValueOnce(
                frontendSession({
                    endpoint: nextEndpoint,
                    websocket: nextWebsocket,
                    currentUserSnapshot: null
                })
            )
            .mockResolvedValueOnce(nextFrontendSession);
        useRuntimeStore.getState().setAuthBootstrap({
            currentUserId: USER_ID,
            currentUserDisplayName: 'Current User',
            currentUserEndpoint: ENDPOINT,
            currentUserWebsocket: WEBSOCKET
        });
        useSessionStore.getState().setSessionState({
            isLoggedIn: true,
            sessionPhase: 'ready'
        });

        await expect(
            resumeFrontendSessionFromBackendRuntime(backendSnapshot())
        ).resolves.toBe(true);

        expect(useRuntimeStore.getState().auth).toMatchObject({
            currentUserId: USER_ID,
            currentUserDisplayName: 'Frontend User',
            currentUserEndpoint: nextEndpoint,
            currentUserWebsocket: nextWebsocket,
            currentUserSnapshot: nextFrontendSession.currentUserSnapshot
        });
        expect(mocks.recordCurrentUserSnapshot).toHaveBeenCalledWith(
            nextFrontendSession.currentUserSnapshot,
            { endpoint: nextEndpoint }
        );
        expect(mocks.loadVrchatConfigSnapshot).toHaveBeenCalledTimes(1);
        expect(mocks.bootstrapAuthenticatedSession).not.toHaveBeenCalled();
        expect(
            mocks.appGetBackendRuntimeFrontendSessionSnapshot.mock.calls
        ).toEqual([[false], [true]]);
    });

    it('bootstraps a matching backend session when the frontend is not ready', async () => {
        useSessionStore.getState().setSessionPhase('authenticated');

        await expect(
            resumeFrontendSessionFromBackendRuntime(backendSnapshot())
        ).resolves.toBe(true);

        expect(useRuntimeStore.getState().auth).toMatchObject({
            currentUserId: USER_ID,
            currentUserDisplayName: 'Frontend User',
            currentUserEndpoint: ENDPOINT,
            currentUserWebsocket: WEBSOCKET,
            currentUserSnapshot: frontendSession().currentUserSnapshot
        });
        expect(mocks.recordCurrentUserSnapshot).toHaveBeenCalledWith(
            frontendSession().currentUserSnapshot,
            { endpoint: ENDPOINT }
        );
        expect(mocks.loadVrchatConfigSnapshot).toHaveBeenCalledTimes(1);
        expect(mocks.bootstrapAuthenticatedSession).toHaveBeenCalledWith(
            frontendSession().currentUserSnapshot,
            expect.any(Number)
        );
        expect(
            mocks.appGetBackendRuntimeFrontendSessionSnapshot
        ).toHaveBeenCalledWith(true);
    });

    it('returns false when bootstrap is superseded by a newer auth attempt', async () => {
        mocks.bootstrapAuthenticatedSession.mockRejectedValueOnce(
            new AuthAttemptSupersededError()
        );

        await expect(
            resumeFrontendSessionFromBackendRuntime(backendSnapshot())
        ).resolves.toBe(false);

        expect(mocks.recordCurrentUserSnapshot).toHaveBeenCalledTimes(1);
        expect(mocks.bootstrapAuthenticatedSession).toHaveBeenCalledTimes(1);
    });
});
