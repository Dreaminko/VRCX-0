import { commands } from '@/platform/tauri/bindings';
import { useDataDirMigrationStore } from '@/state/dataDirMigrationStore';
import { useProfileBackupStore } from '@/state/profileBackupStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';

import { handleAppLauncherSnapshotEvent } from './appLauncherSnapshotService';
import {
    applyAuthenticatedRuntimePhaseSnapshot,
    handleAuthenticatedRuntimeRealtimeStatus,
    matchesAuthenticatedRuntimeAuthFailure,
    resetAuthenticatedRuntimeMirror
} from './authenticatedRuntimeService';
import { handleRuntimeAuthFailure } from './authSessionRecoveryService';
import { applyBackgroundImageProjectionEvent } from './background-image/backgroundImageService';
import { handleAppUpdateStatusEvent } from './backgroundMaintenanceUpdateService';
import {
    applyCommunityThemeProjectionEvent,
    refreshCommunityThemeProjection
} from './community-theme/installedThemes';
import { getCurrentDataDirMigrationStatus } from './dataDirMigrationService';
import { bindDeepLinkEvents, drainPendingDeepLinks } from './deepLinkService';
import { handleFavoriteImportStatusEvent } from './favoriteImportService';
import { applyFriendProfileLoadStatusPayload } from './friendProfileLoadService';
import { handleGroupBanImportStatusEvent } from './groupBanImportService';
import {
    handleMutualGraphFetchStatusEvent,
    refreshMutualGraphFetchStatus
} from './mutualGraphFetchService';
import { getCurrentProfileBackupStatus } from './profileBackupService';
import { handleRealtimeEntryCorrection } from './realtimePresenceService';
import { runForegroundUpdateRegistryBackupMaintenance } from './registryBackupMaintenanceService';
import {
    handleFavoritesChangedEvent,
    handlePrintCleanupEvent,
    handleRuntimeGroupInstancesProjection,
    requestGroupInstancesRefresh
} from './runtime-event-bridge/auxiliaryEventHandlers';
import {
    flushPendingBackendRealtimeProjectionEvents,
    handleBackendRealtimeProjectionEvent,
    prunePendingBackendRealtimeProjectionEvents,
    resetBackendRealtimeProjectionState
} from './runtime-event-bridge/backendRealtimeProjection';
import {
    handleBackendRuntimeSyncSnapshot,
    hydrateBackendRuntimeSnapshot
} from './runtime-event-bridge/backendRuntimeHydration';
import {
    handleBrowserFocusEvent,
    handleDebugLoggingOutcome,
    handleGameClientEvent,
    handleGameLogPersistenceFallback,
    handleGameLogSideEffect,
    handleRuntimeGameLogProjection,
    handleUpdateIsGameRunning
} from './runtime-event-bridge/gameRuntimeEventHandlers';
import { subscribeRuntimeEvent as subscribeTypedRuntimeEvent } from './runtime-event-bridge/subscription';
import type {
    RuntimeEvent,
    RuntimeEventName,
    RuntimeEventPayloadMap
} from './runtime-event-bridge/types';
import { handleScreenshotLibraryScanStatusEvent } from './screenshotLibraryScanService';
import {
    handleAppUpdateDownloadProgressEvent,
    handleAppUpdateInstalledEvent
} from './updateInstallService';
import { applyVrcStatusSnapshot } from './vrcStatusService';

type RuntimeEventUnsubscribe = () => void;

function handleRuntimeVrchatAuthFailureEvent(
    failure: RuntimeEventPayloadMap['runtimeVrchatAuthFailure']
): void {
    if (!matchesAuthenticatedRuntimeAuthFailure(failure)) {
        return;
    }
    void handleRuntimeAuthFailure(failure);
}

function handleRuntimeEvent(event: RuntimeEvent): void {
    const runtimeStore = useRuntimeStore.getState();

    if (event.name === 'gameLogPersistenceFallback') {
        handleGameLogPersistenceFallback(event.payload);
        return;
    }

    if (event.name === 'friendProfileLoadStatus') {
        runtimeStore.recordRuntimeEvent(event.name, event.payload);
        applyFriendProfileLoadStatusPayload(event.payload);
        return;
    }

    if (event.name === 'printsAutoCleanup') {
        runtimeStore.recordRuntimeEvent(event.name, event.payload);
        handlePrintCleanupEvent(event.payload);
        return;
    }

    if (event.name === 'appUpdateStatus') {
        void handleAppUpdateStatusEvent(event.payload);
        void runForegroundUpdateRegistryBackupMaintenance();
        return;
    }

    if (event.name === 'appLauncherSnapshot') {
        handleAppLauncherSnapshotEvent(event.payload);
        return;
    }

    if (event.name === 'appUpdateDownloadProgress') {
        handleAppUpdateDownloadProgressEvent(event.payload);
        return;
    }

    if (event.name === 'appUpdateInstalled') {
        handleAppUpdateInstalledEvent(event.payload);
        return;
    }

    if (event.name === 'profileBackupStatus') {
        useProfileBackupStore.getState().applyStatus(event.payload);
        return;
    }

    if (event.name === 'profileRestoreProgress') {
        useProfileBackupStore.getState().applyRestoreProgress(event.payload);
        return;
    }

    if (event.name === 'dataDirMigration') {
        useDataDirMigrationStore.getState().applyStatus(event.payload);
        return;
    }

    if (event.name === 'backgroundImageState') {
        applyBackgroundImageProjectionEvent(event.payload);
        return;
    }

    if (event.name === 'communityThemeState') {
        applyCommunityThemeProjectionEvent(event.payload);
        return;
    }

    if (event.name === 'vrcStatus') {
        applyVrcStatusSnapshot(event.payload);
        return;
    }

    if (event.name === 'favoriteImportStatus') {
        handleFavoriteImportStatusEvent(event.payload);
        return;
    }

    if (event.name === 'groupBanImportStatus') {
        handleGroupBanImportStatusEvent(event.payload);
        return;
    }

    if (event.name === 'mutualGraphFetchStatus') {
        handleMutualGraphFetchStatusEvent(event.payload);
        return;
    }

    if (event.name === 'screenshotLibraryScanStatus') {
        handleScreenshotLibraryScanStatusEvent(event.payload);
        return;
    }

    if (event.name === 'favoritesChanged') {
        runtimeStore.recordRuntimeEvent(event.name, event.payload);
        handleFavoritesChangedEvent(event.payload);
        return;
    }

    if (event.name === 'authenticatedRuntimePhase') {
        runtimeStore.recordRuntimeEvent(event.name, event.payload);
        applyAuthenticatedRuntimePhaseSnapshot(event.payload);
        return;
    }

    if (event.name === 'realtimeWsStatus') {
        handleAuthenticatedRuntimeRealtimeStatus(event.payload);
        return;
    }

    if (event.name === 'realtimeProjectionSync') {
        const snapshot = event.payload.snapshot;
        prunePendingBackendRealtimeProjectionEvents(snapshot);
        handleBackendRuntimeSyncSnapshot(
            snapshot,
            flushPendingBackendRealtimeProjectionEvents
        );
        return;
    }

    if (handleBackendRealtimeProjectionEvent(event)) {
        return;
    }

    runtimeStore.recordRuntimeEvent(event.name, event.payload);

    if (event.name === 'realtimeEntryCorrection') {
        handleRealtimeEntryCorrection(event.payload);
        return;
    }

    if (event.name === 'gameLogProjection') {
        handleRuntimeGameLogProjection(event.payload);
        return;
    }

    if (event.name === 'gameLogSideEffect') {
        handleGameLogSideEffect(event.payload);
        return;
    }

    if (event.name === 'runtimeGroupInstancesProjection') {
        handleRuntimeGroupInstancesProjection(event.payload);
        return;
    }

    if (event.name === 'gameClientEvent') {
        handleGameClientEvent(event.payload);
        return;
    }

    if (event.name === 'runtimeWorkerError') {
        console.warn('Backend worker error:', event.payload);
        return;
    }

    if (event.name === 'runtimeVrchatAuthFailure') {
        handleRuntimeVrchatAuthFailureEvent(event.payload);
        return;
    }

    if (event.name === 'updateIsGameRunning') {
        handleUpdateIsGameRunning(event.payload);
        return;
    }

    if (event.name === 'browserFocus') {
        handleBrowserFocusEvent();
    }
}

async function hydrateRuntimeState(
    failureMessage: string,
    hydrate: () => Promise<unknown>
): Promise<void> {
    try {
        await hydrate();
    } catch (error) {
        console.warn(failureMessage, error);
    }
}

export async function bindRuntimeEvents(): Promise<() => void> {
    resetBackendRealtimeProjectionState();
    resetAuthenticatedRuntimeMirror();
    const unsubscribers: RuntimeEventUnsubscribe[] = [];
    const events: RuntimeEventName[] = [
        'addGameLogEvent',
        'authenticatedRuntimePhase',
        'appUpdateStatus',
        'appUpdateDownloadProgress',
        'appUpdateInstalled',
        'appLauncherSnapshot',
        'backendRuntimeTelemetry',
        'backgroundImageState',
        'communityThemeState',
        'gameLogProjection',
        'gameLogPersistenceFallback',
        'gameLogSideEffect',
        'runtimeGroupInstancesProjection',
        'overlayActivitySnapshot',
        'printsAutoCleanup',
        'profileBackupStatus',
        'profileRestoreProgress',
        'dataDirMigration',
        'favoriteImportStatus',
        'favoritesChanged',
        'groupBanImportStatus',
        'groupModerationBatchProgress',
        'mutualGraphFetchStatus',
        'screenshotLibraryScanStatus',
        'friendProfileLoadStatus',
        'gameClientEvent',
        'runtimeWorkerError',
        'runtimeVrchatAuthFailure',
        'vrcStatus',
        'realtimeFriendProjection',
        'realtimeUserProjection',
        'realtimeEntryCorrection',
        'realtimeNotificationProjection',
        'realtimeWsStatus',
        'realtimeCurrentUserProjection',
        'realtimeInstanceClosedProjection',
        'realtimeInstanceQueueProjection',
        'realtimeProjectionSync',
        'updateIsGameRunning',
        'browserFocus'
    ];

    useSessionStore.getState().setTransportStatus('runtime-subscribing');

    try {
        const subscriptions = await Promise.allSettled(
            events.map(subscribeRuntimeEvent)
        );
        const failure = subscriptions.find(
            (subscription) => subscription.status === 'rejected'
        );
        for (const subscription of subscriptions) {
            if (subscription.status === 'fulfilled') {
                unsubscribers.push(subscription.value);
            }
        }
        if (failure) {
            throw failure.reason;
        }
        await Promise.all([
            hydrateRuntimeState(
                'Failed to hydrate community theme projection:',
                refreshCommunityThemeProjection
            ),
            hydrateRuntimeState(
                'Failed to hydrate profile backup status:',
                async () => {
                    useProfileBackupStore
                        .getState()
                        .applyStatus(await getCurrentProfileBackupStatus());
                }
            ),
            hydrateRuntimeState(
                'Failed to hydrate data directory migration status:',
                async () => {
                    useDataDirMigrationStore
                        .getState()
                        .applyStatus(await getCurrentDataDirMigrationStatus());
                }
            ),
            hydrateRuntimeState(
                'Failed to hydrate mutual graph fetch status:',
                refreshMutualGraphFetchStatus
            ),
            hydrateRuntimeState(
                'Failed to hydrate app update status:',
                async () => {
                    await handleAppUpdateStatusEvent(
                        await commands.appAppUpdateStatusGet()
                    );
                }
            ),
            hydrateRuntimeState(
                'Failed to hydrate debug logging status:',
                async () => {
                    const debugLoggingOutcome =
                        await commands.appGameClientDebugLoggingStatus();
                    if (debugLoggingOutcome) {
                        handleDebugLoggingOutcome(debugLoggingOutcome);
                    }
                }
            ),
            hydrateRuntimeState(
                'Failed to hydrate background image state:',
                async () => {
                    applyBackgroundImageProjectionEvent(
                        await commands.appBackgroundImageStateGet()
                    );
                }
            ),
            hydrateRuntimeState(
                'Failed to run registry backup maintenance during hydration:',
                runForegroundUpdateRegistryBackupMaintenance
            ),
            hydrateRuntimeState(
                'Failed to hydrate app update download status:',
                async () => {
                    const downloadStatus =
                        await commands.appAppUpdateDownloadStatusGet();
                    useRuntimeStore.getState().setUpdateLoopState({
                        autoDownloadState: downloadStatus.phase,
                        downloadedVersion: downloadStatus.version,
                        downloadProgress: downloadStatus.percent
                    });
                }
            )
        ]);
    } catch (error) {
        resetBackendRealtimeProjectionState();
        resetAuthenticatedRuntimeMirror();
        unsubscribeRuntimeEvents(unsubscribers);
        useRuntimeStore.getState().setShellState({
            backendRuntimeSnapshotHydrated: true,
            backendRuntimeSessionHydrating: false
        });
        useSessionStore.getState().setTransportStatus('disconnected');
        throw error;
    }

    useSessionStore.getState().setTransportStatus('runtime-subscribed');
    try {
        const snapshot = await commands.appGetBackendRuntimeSnapshot();
        await hydrateBackendRuntimeSnapshot(
            snapshot,
            flushPendingBackendRealtimeProjectionEvents
        );
    } catch (error) {
        useRuntimeStore.getState().setShellState({
            backendRuntimeSnapshotHydrated: true,
            backendRuntimeSessionHydrating: false
        });
        console.warn('Failed to hydrate backend runtime snapshot:', error);
    }
    try {
        applyAuthenticatedRuntimePhaseSnapshot(
            await commands.appAuthenticatedRuntimePhaseSnapshotGet()
        );
    } catch (error) {
        console.warn('Failed to hydrate authenticated runtime phase:', error);
    }
    try {
        unsubscribers.push(await bindDeepLinkEvents());
        await drainPendingDeepLinks();
    } catch (error) {
        resetBackendRealtimeProjectionState();
        resetAuthenticatedRuntimeMirror();
        unsubscribeRuntimeEvents(unsubscribers);
        useSessionStore.getState().setTransportStatus('disconnected');
        throw error;
    }
    requestGroupInstancesRefresh(
        'runtime event binding after backend snapshot hydration'
    );

    return () => {
        resetBackendRealtimeProjectionState();
        resetAuthenticatedRuntimeMirror();
        unsubscribeRuntimeEvents(unsubscribers);
        useSessionStore.getState().setTransportStatus('disconnected');
    };
}

function unsubscribeRuntimeEvents(
    unsubscribers: RuntimeEventUnsubscribe[]
): void {
    for (const unsubscribe of unsubscribers) {
        if (typeof unsubscribe === 'function') {
            unsubscribe();
        }
    }
}

function subscribeRuntimeEvent<Name extends RuntimeEventName>(
    name: Name
): Promise<RuntimeEventUnsubscribe> {
    return subscribeTypedRuntimeEvent(name, (payload) => {
        handleRuntimeEvent({ name, payload } as RuntimeEvent);
    });
}
