import { commands } from '@/platform/tauri/bindings';
import type {
    GameLogProjection,
    HostSessionProjection
} from '@/platform/tauri/bindings';
import { normalizeString } from '@/shared/utils/string';
import { useNotificationStore } from '@/state/notificationStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import { recordRuntimeGameClientEvent } from '../gameClientLifecycle';
import { applyRuntimeGameLogProjection } from '../gameLogIngestService';
import { handleGameRunningUpdate } from '../gameStateService';
import { isHostCapabilityAvailable } from '../hostCapabilityService';
import { pushSharedFeedNotification } from '../sharedFeedNotificationService';
import { handleBrowserFocus } from '../vrcStatusService';
import { isRecord } from './guards';

function publishNowPlayingSharedFeed(payload: Record<string, unknown>): void {
    const videoUrl = normalizeString(payload.videoUrl || payload.url);
    if (!videoUrl) {
        return;
    }

    const videoName = normalizeString(payload.videoName || payload.name);
    const displayName = normalizeString(payload.displayName);
    const message = [
        videoName || videoUrl,
        displayName ? `(${displayName})` : ''
    ]
        .filter(Boolean)
        .join(' ');

    pushSharedFeedNotification({
        ...payload,
        created_at:
            normalizeString(payload.created_at) ||
            normalizeString(payload.startedAt) ||
            new Date().toISOString(),
        type: 'VideoPlay',
        videoUrl,
        videoName,
        videoId: normalizeString(payload.videoId || payload.source),
        location: normalizeString(payload.location),
        displayName,
        userId: normalizeString(payload.userId),
        message,
        notyName: message
    }).catch((error: unknown) => {
        console.warn(
            'Failed to publish runtime video shared feed notification:',
            error
        );
    });
}

function requestGameRunningStateRefresh(source: string): void {
    if (!isHostCapabilityAvailable('gameProcessMonitor')) {
        return;
    }

    commands.appCheckGameRunning().catch((error: unknown) => {
        console.warn(
            `Game process state refresh failed during ${source}:`,
            error
        );
    });
}

export function handleGameLogPersistenceFallback(payload: unknown): void {
    useRuntimeStore
        .getState()
        .recordRuntimeEvent('gameLogPersistenceFallback', payload);
    const record = isRecord(payload) ? payload : {};
    const errorMessage = normalizeString(record.error);
    if (errorMessage) {
        console.warn('Backend GameLog persistence failed:', errorMessage);
    }
}

export function handleRuntimeGameLogProjection(
    payload: GameLogProjection
): void {
    if (!isHostCapabilityAvailable('runtimeGameLogIngest')) {
        return;
    }
    applyRuntimeGameLogProjection(payload);
}

export function handleGameLogSideEffect(payload: unknown): void {
    if (!isHostCapabilityAvailable('runtimeGameLogSideEffects')) {
        return;
    }
    const runtimeStore = useRuntimeStore.getState();
    const record = isRecord(payload) ? payload : {};
    const kind = String(record.kind || '');
    const sidePayload = isRecord(record.payload) ? record.payload : {};
    if (kind === 'nowPlaying') {
        runtimeStore.setNowPlayingState(sidePayload);
        publishNowPlayingSharedFeed(sidePayload);
    } else if (kind === 'nowPlayingReset') {
        runtimeStore.resetNowPlayingState();
    } else if (kind === 'screenshotProcessed') {
        runtimeStore.setGameState({
            lastScreenshotPath: String(sidePayload.path || '')
        });
    } else if (kind === 'gameNoVR') {
        runtimeStore.setGameState({
            isGameNoVR: Boolean(sidePayload.isGameNoVR)
        });
    } else if (kind === 'notification') {
        useNotificationStore.getState().pushNotification(sidePayload);
    }
}

export function handleGameClientEvent(payload: unknown): void {
    if (!isHostCapabilityAvailable('runtimeGameClientLifecycle')) {
        return;
    }
    const record = isRecord(payload) ? payload : {};
    const kind = String(record.kind || '');
    const clientPayload = isRecord(record.payload) ? record.payload : {};
    recordRuntimeGameClientEvent(kind, clientPayload);
    if (kind === 'notification') {
        useNotificationStore.getState().pushNotification(clientPayload);
    }
}

export function handleUpdateIsGameRunning(
    payload: HostSessionProjection
): void {
    if (!isHostCapabilityAvailable('gameProcessMonitor')) {
        return;
    }
    handleGameRunningUpdate(payload).catch((error: unknown) => {
        useNotificationStore.getState().pushNotification({
            level: 'warning',
            title: 'Game state update failed',
            message: error instanceof Error ? error.message : String(error)
        });
    });
}

export function handleBrowserFocusEvent(): void {
    useRuntimeStore.getState().setGameState({
        lastBrowserFocusAt: new Date().toISOString()
    });
    requestGameRunningStateRefresh('browser focus');
    handleBrowserFocus().catch((error: unknown) => {
        console.warn('Browser focus status refresh failed:', error);
    });
}
