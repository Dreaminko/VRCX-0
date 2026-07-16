import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    setTrayIconNotification: vi.fn(),
    startRuntimeAuthFailureRecovery: vi.fn(),
    bootstrapFavorites: vi.fn(),
    bootstrapFriendRoster: vi.fn(),
    startRuntimeGameClientSync: vi.fn(),
    stopGameStateService: vi.fn(),
    getTimeUnitLabels: vi.fn(),
    setI18nLanguage: vi.fn(),
    startRealtimeTransport: vi.fn(),
    stopRealtimeTransport: vi.fn(),
    bindRuntimeEvents: vi.fn(),
    initializeReactRuntime: vi.fn(),
    syncStartupServicesTask: vi.fn(),
    applyThemeMode: vi.fn(),
    startRuntimeUpdateLoop: vi.fn(),
    startVrcStatusPolling: vi.fn()
}));

vi.mock('@/services/shellIntegrationService', () => ({
    setTrayIconNotification: mocks.setTrayIconNotification
}));

vi.mock('./authSessionRecoveryService', () => ({
    startRuntimeAuthFailureRecovery: mocks.startRuntimeAuthFailureRecovery
}));

vi.mock('./favoriteBootstrapService', () => ({
    bootstrapFavorites: mocks.bootstrapFavorites
}));

vi.mock('./friendBootstrapService', () => ({
    bootstrapFriendRoster: mocks.bootstrapFriendRoster
}));

vi.mock('./gameClientLifecycle', () => ({
    startRuntimeGameClientSync: mocks.startRuntimeGameClientSync
}));

vi.mock('./gameStateService', () => ({
    stopGameStateService: mocks.stopGameStateService
}));

vi.mock('./i18nService', () => ({
    getTimeUnitLabels: mocks.getTimeUnitLabels,
    setI18nLanguage: mocks.setI18nLanguage
}));

vi.mock('./realtimeTransportService', () => ({
    startRealtimeTransport: mocks.startRealtimeTransport,
    stopRealtimeTransport: mocks.stopRealtimeTransport
}));

vi.mock('./runtimeEventBridgeService', () => ({
    bindRuntimeEvents: mocks.bindRuntimeEvents
}));

vi.mock('./startupService', () => ({
    initializeReactRuntime: mocks.initializeReactRuntime
}));

vi.mock('./startupServicesStatus', () => ({
    syncStartupServicesTask: mocks.syncStartupServicesTask
}));

vi.mock('./themeService', () => ({
    applyThemeMode: mocks.applyThemeMode
}));

vi.mock('./updateLoopService', () => ({
    startRuntimeUpdateLoop: mocks.startRuntimeUpdateLoop
}));

vi.mock('./vrcStatusService', () => ({
    startVrcStatusPolling: mocks.startVrcStatusPolling
}));

import { useNotificationStore } from '@/state/notificationStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';
import { DEFAULT_TIME_UNIT_LABELS, useShellStore } from '@/state/shellStore';

import {
    startAuthenticatedRuntimeServices,
    startI18nLanguageSync,
    startReactRuntimeServices
} from './runtimeBootstrapService';

type Deferred<T> = {
    promise: Promise<T>;
    resolve: (value: T) => void;
    reject: (error: unknown) => void;
};

function deferred<T>(): Deferred<T> {
    let resolve!: (value: T) => void;
    let reject!: (error: unknown) => void;
    const promise = new Promise<T>((promiseResolve, promiseReject) => {
        resolve = promiseResolve;
        reject = promiseReject;
    });
    return { promise, resolve, reject };
}

function installDocumentStub() {
    globalThis.document = {
        documentElement: {
            setAttribute: vi.fn()
        }
    } as unknown as Document;
}

function installWindowStub() {
    globalThis.window = {
        setTimeout: globalThis.setTimeout,
        clearTimeout: globalThis.clearTimeout
    } as unknown as Window & typeof globalThis;
}

function resetShellStore() {
    useShellStore.setState({
        locale: 'en',
        timeUnitLabels: DEFAULT_TIME_UNIT_LABELS
    });
}

function setAuthenticatedContext(
    userId: string,
    options: { friendsLoaded?: boolean; favoritesLoaded?: boolean } = {}
) {
    useSessionStore.getState().setSessionState({
        sessionPhase: 'ready',
        isLoggedIn: true,
        isFriendsLoaded: options.friendsLoaded ?? false,
        isFavoritesLoaded: options.favoritesLoaded ?? false
    });
    useRuntimeStore.getState().setAuthBootstrap({
        currentUserId: userId,
        currentUserEndpoint: `https://api.example/${userId}`,
        currentUserWebsocket: `wss://ws.example/${userId}`,
        currentUserSnapshot: { id: userId }
    });
}

describe('runtimeBootstrapService', () => {
    beforeEach(() => {
        installDocumentStub();
        installWindowStub();
        vi.clearAllMocks();
        useRuntimeStore.getState().resetRuntimeState();
        useSessionStore.getState().resetSessionState();
        useNotificationStore.getState().resetNotificationState();
        resetShellStore();
        mocks.getTimeUnitLabels.mockImplementation(
            (locale: string, fallback: typeof DEFAULT_TIME_UNIT_LABELS) => ({
                ...fallback,
                h: `${locale}:h`
            })
        );
        mocks.setI18nLanguage.mockResolvedValue(undefined);
        mocks.bootstrapFriendRoster.mockResolvedValue(undefined);
        mocks.bootstrapFavorites.mockResolvedValue(undefined);
        mocks.startRealtimeTransport.mockResolvedValue(undefined);
        mocks.initializeReactRuntime.mockResolvedValue(undefined);
        mocks.bindRuntimeEvents.mockResolvedValue(undefined);
        mocks.startRuntimeAuthFailureRecovery.mockReturnValue(undefined);
        mocks.startRuntimeGameClientSync.mockReturnValue(undefined);
        mocks.startRuntimeUpdateLoop.mockReturnValue(undefined);
        mocks.startVrcStatusPolling.mockReturnValue(undefined);
    });

    afterEach(() => {
        vi.useRealTimers();
    });

    it('syncs normalized locale state', () => {
        useShellStore.getState().setLocale('zh_Hant_TW');

        const cleanup = startI18nLanguageSync();

        expect(document.documentElement.setAttribute).toHaveBeenCalledWith(
            'lang',
            'zh-TW'
        );
        expect(mocks.setI18nLanguage).toHaveBeenCalledWith('zh-TW');
        expect(useShellStore.getState().timeUnitLabels.h).toBe('zh-TW:h');

        useShellStore.getState().setLocale('en-US');

        expect(document.documentElement.setAttribute).toHaveBeenLastCalledWith(
            'lang',
            'en'
        );
        expect(mocks.setI18nLanguage).toHaveBeenLastCalledWith('en');
        expect(useShellStore.getState().timeUnitLabels.h).toBe('en:h');

        cleanup();
        useShellStore.getState().setLocale('zh_CN');
        expect(mocks.setI18nLanguage).toHaveBeenCalledTimes(2);
    });

    it('starts realtime only after authenticated bootstraps', () => {
        const currentUserSnapshot = {
            id: 'usr_self',
            displayName: 'Current User'
        };
        useSessionStore.getState().setSessionState({
            sessionPhase: 'ready',
            isLoggedIn: true,
            isFriendsLoaded: false,
            isFavoritesLoaded: false
        });
        useRuntimeStore.getState().setAuthBootstrap({
            currentUserId: 'usr_self',
            currentUserEndpoint: 'https://api.example',
            currentUserWebsocket: 'wss://ws.example',
            currentUserSnapshot
        });

        const cleanup = startAuthenticatedRuntimeServices();

        expect(mocks.bootstrapFriendRoster).toHaveBeenCalledWith({
            userId: 'usr_self',
            endpoint: 'https://api.example',
            currentUserSnapshot
        });
        expect(mocks.bootstrapFavorites).toHaveBeenCalledWith({
            userId: 'usr_self',
            endpoint: 'https://api.example',
            currentUserSnapshot
        });
        expect(mocks.startRealtimeTransport).not.toHaveBeenCalled();

        useSessionStore.getState().setFriendsLoaded(true);

        expect(mocks.startRealtimeTransport).toHaveBeenCalledWith({
            userId: 'usr_self',
            endpoint: 'https://api.example',
            websocket: 'wss://ws.example',
            currentUserSnapshot
        });

        cleanup();
        expect(mocks.stopRealtimeTransport.mock.calls.at(-1)).toEqual([]);
    });

    it('retries realtime startup after the backoff without external updates', async () => {
        vi.useFakeTimers();
        installWindowStub();
        const firstStart = deferred<void>();
        const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
        mocks.startRealtimeTransport
            .mockReturnValueOnce(firstStart.promise)
            .mockResolvedValueOnce(undefined);
        setAuthenticatedContext('usr_self', {
            friendsLoaded: true,
            favoritesLoaded: true
        });

        const cleanup = startAuthenticatedRuntimeServices();
        expect(mocks.startRealtimeTransport).toHaveBeenCalledTimes(1);
        firstStart.reject(new Error('realtime failed'));
        await vi.advanceTimersByTimeAsync(0);
        expect(warn).toHaveBeenCalledTimes(1);
        expect(vi.getTimerCount()).toBe(1);

        await vi.advanceTimersByTimeAsync(4_999);
        expect(mocks.startRealtimeTransport).toHaveBeenCalledTimes(1);
        await vi.advanceTimersByTimeAsync(1);
        expect(mocks.startRealtimeTransport).toHaveBeenCalledTimes(2);

        cleanup();
        warn.mockRestore();
    });

    it('clears bootstrap retry timers when the account changes', async () => {
        vi.useFakeTimers();
        installWindowStub();
        mocks.bootstrapFriendRoster
            .mockRejectedValueOnce(new Error('friends failed'))
            .mockResolvedValue(undefined);
        mocks.bootstrapFavorites
            .mockRejectedValueOnce(new Error('favorites failed'))
            .mockResolvedValue(undefined);
        setAuthenticatedContext('usr_a');

        const cleanup = startAuthenticatedRuntimeServices();
        await vi.runAllTicks();
        await Promise.resolve();

        setAuthenticatedContext('usr_b');
        await vi.runAllTicks();
        await Promise.resolve();

        expect(mocks.bootstrapFriendRoster).toHaveBeenCalledTimes(2);
        expect(mocks.bootstrapFavorites).toHaveBeenCalledTimes(2);
        expect(mocks.bootstrapFriendRoster.mock.calls[1]?.[0].userId).toBe(
            'usr_b'
        );
        expect(mocks.bootstrapFavorites.mock.calls[1]?.[0].userId).toBe(
            'usr_b'
        );
        expect(vi.getTimerCount()).toBe(0);

        await vi.advanceTimersByTimeAsync(60_000);
        expect(mocks.bootstrapFriendRoster).toHaveBeenCalledTimes(2);
        expect(mocks.bootstrapFavorites).toHaveBeenCalledTimes(2);
        cleanup();
    });

    it('hands realtime ownership between backend and frontend runtimes', () => {
        setAuthenticatedContext('usr_self', {
            friendsLoaded: true,
            favoritesLoaded: true
        });
        useRuntimeStore.getState().setBackendRuntimeSnapshot({
            phase: 'running',
            authStatus: 'authenticated',
            authUserId: 'usr_self',
            wsStatus: 'connected',
            mode: 'background'
        });

        const cleanup = startAuthenticatedRuntimeServices();

        expect(mocks.startRealtimeTransport).not.toHaveBeenCalled();
        expect(useSessionStore.getState().transportStatus).toBe(
            'pipeline-connected'
        );
        expect(mocks.syncStartupServicesTask).toHaveBeenCalledWith([
            'Backend realtime transport is active.'
        ]);

        useRuntimeStore.getState().setBackendRuntimeSnapshot({
            phase: 'running',
            authStatus: 'authenticated',
            authUserId: 'usr_self',
            wsStatus: 'connected',
            mode: 'foreground'
        });
        expect(mocks.startRealtimeTransport).toHaveBeenCalledTimes(1);

        useRuntimeStore.getState().setBackendRuntimeSnapshot({
            phase: 'running',
            authStatus: 'authenticated',
            authUserId: 'usr_self',
            wsStatus: 'connected',
            mode: 'background'
        });
        expect(mocks.stopRealtimeTransport).toHaveBeenCalledWith({
            updateStatus: false
        });
        cleanup();
    });

    it('ignores stale frontend rejection after a newer start', async () => {
        const firstStart = deferred<void>();
        const secondStart = deferred<void>();
        const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
        mocks.startRealtimeTransport
            .mockReturnValueOnce(firstStart.promise)
            .mockReturnValueOnce(secondStart.promise);
        setAuthenticatedContext('usr_self', {
            friendsLoaded: true,
            favoritesLoaded: true
        });

        const cleanup = startAuthenticatedRuntimeServices();
        expect(mocks.startRealtimeTransport).toHaveBeenCalledTimes(1);

        useRuntimeStore.getState().setBackendRuntimeSnapshot({
            phase: 'running',
            authStatus: 'authenticated',
            authUserId: 'usr_self',
            wsStatus: 'connected',
            mode: 'background'
        });
        useRuntimeStore.getState().setBackendRuntimeSnapshot({
            phase: 'running',
            authStatus: 'authenticated',
            authUserId: 'usr_self',
            wsStatus: 'connected',
            mode: 'foreground'
        });
        expect(mocks.startRealtimeTransport).toHaveBeenCalledTimes(2);

        firstStart.reject(new Error('stale start failed'));
        await Promise.resolve();
        await Promise.resolve();
        expect(warn).not.toHaveBeenCalled();

        const stopCountBeforeTakeover =
            mocks.stopRealtimeTransport.mock.calls.length;
        useRuntimeStore.getState().setBackendRuntimeSnapshot({
            phase: 'running',
            authStatus: 'authenticated',
            authUserId: 'usr_self',
            wsStatus: 'connected',
            mode: 'background'
        });
        expect(mocks.stopRealtimeTransport).toHaveBeenCalledTimes(
            stopCountBeforeTakeover + 1
        );
        expect(mocks.stopRealtimeTransport).toHaveBeenLastCalledWith({
            updateStatus: false
        });

        secondStart.resolve();
        cleanup();
        warn.mockRestore();
    });

    it('shares React runtime startup across consumers', async () => {
        const initialization = deferred<void>();
        const authCleanup = vi.fn();
        const eventCleanup = vi.fn();
        const gameClientCleanup = vi.fn();
        const updateLoopCleanup = vi.fn();
        const statusCleanup = vi.fn();
        mocks.initializeReactRuntime.mockReturnValue(initialization.promise);
        mocks.startRuntimeAuthFailureRecovery.mockReturnValue(authCleanup);
        mocks.bindRuntimeEvents.mockResolvedValue(eventCleanup);
        mocks.startRuntimeGameClientSync.mockReturnValue(gameClientCleanup);
        mocks.startRuntimeUpdateLoop.mockReturnValue(updateLoopCleanup);
        mocks.startVrcStatusPolling.mockReturnValue(statusCleanup);

        const cleanupFirst = startReactRuntimeServices();
        const cleanupSecond = startReactRuntimeServices();
        expect(mocks.initializeReactRuntime).toHaveBeenCalledTimes(1);

        initialization.resolve();
        await vi.waitFor(() =>
            expect(mocks.bindRuntimeEvents).toHaveBeenCalled()
        );

        cleanupFirst();
        expect(authCleanup).not.toHaveBeenCalled();
        cleanupSecond();

        expect(authCleanup).toHaveBeenCalledTimes(1);
        expect(eventCleanup).toHaveBeenCalledTimes(1);
        expect(gameClientCleanup).toHaveBeenCalledTimes(1);
        expect(updateLoopCleanup).toHaveBeenCalledTimes(1);
        expect(statusCleanup).toHaveBeenCalledTimes(1);
        expect(mocks.stopGameStateService).toHaveBeenCalledTimes(1);
    });

    it('cleans up runtime startup after its consumer leaves', async () => {
        const initialization = deferred<void>();
        const authCleanup = vi.fn();
        const eventCleanup = vi.fn();
        mocks.initializeReactRuntime.mockReturnValue(initialization.promise);
        mocks.startRuntimeAuthFailureRecovery.mockReturnValue(authCleanup);
        mocks.bindRuntimeEvents.mockResolvedValue(eventCleanup);

        const cleanup = startReactRuntimeServices();
        cleanup();
        initialization.resolve();

        await vi.waitFor(() => expect(eventCleanup).toHaveBeenCalledTimes(1));
        expect(authCleanup).toHaveBeenCalledTimes(1);
        expect(mocks.stopGameStateService).toHaveBeenCalledTimes(1);
    });
});
