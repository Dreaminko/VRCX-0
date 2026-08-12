import { toast } from 'sonner';

import type {
    LocalApiStartFailedPayload,
    LocalApiStatus
} from '@/platform/tauri/bindings';

import i18n from './i18nService';

type LocalApiStatusRefreshListener = () => void;

const statusRefreshListeners = new Set<LocalApiStatusRefreshListener>();
let lastPresentedFailure: { key: string; at: number } | null = null;

export function handleLocalApiStartFailed(
    failure: LocalApiStartFailedPayload
): void {
    presentStartFailure(failure);
    requestLocalApiStatusRefresh();
}

export function hydrateLocalApiStatus(status: LocalApiStatus): void {
    if (status.state !== 'error' || !status.lastError) {
        lastPresentedFailure = null;
        return;
    }
    presentStartFailure({
        port: status.lastError.port ?? status.port,
        reason: status.lastError.code === 'portInUse' ? 'portInUse' : 'bind'
    });
    requestLocalApiStatusRefresh();
}

export function requestLocalApiStatusRefresh(): void {
    for (const listener of statusRefreshListeners) {
        listener();
    }
}

export function subscribeLocalApiStatusRefresh(
    listener: LocalApiStatusRefreshListener
): () => void {
    statusRefreshListeners.add(listener);
    return () => {
        statusRefreshListeners.delete(listener);
    };
}

function presentStartFailure(failure: LocalApiStartFailedPayload): void {
    const key = `${failure.reason}:${failure.port}`;
    const now = Date.now();
    if (
        lastPresentedFailure?.key === key &&
        now - lastPresentedFailure.at < 5000
    ) {
        return;
    }
    lastPresentedFailure = { key, at: now };
    const reasonKey =
        failure.reason === 'portInUse'
            ? 'view.settings.integrations.local_api.port_in_use'
            : 'view.settings.integrations.local_api.bind_failed';
    const reason = i18n.t(reasonKey, { port: failure.port });
    toast.error(
        i18n.t('view.settings.integrations.local_api.start_failed', {
            reason
        })
    );
}
