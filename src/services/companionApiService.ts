import { toast } from 'sonner';

import type {
    CompanionApiStartFailedPayload,
    CompanionApiStatus
} from '@/platform/tauri/bindings';

import i18n from './i18nService';

type CompanionApiStatusRefreshListener = () => void;

const statusRefreshListeners = new Set<CompanionApiStatusRefreshListener>();
let lastPresentedFailure: { key: string; at: number } | null = null;

export function handleCompanionApiStartFailed(
    failure: CompanionApiStartFailedPayload
): void {
    presentStartFailure(failure);
    requestCompanionApiStatusRefresh();
}

export function hydrateCompanionApiStatus(status: CompanionApiStatus): void {
    if (status.state !== 'error' || !status.lastError) {
        lastPresentedFailure = null;
        return;
    }
    presentStartFailure({
        port: status.lastError.port ?? status.port,
        reason: status.lastError.code === 'portInUse' ? 'portInUse' : 'bind'
    });
    requestCompanionApiStatusRefresh();
}

export function requestCompanionApiStatusRefresh(): void {
    for (const listener of statusRefreshListeners) {
        listener();
    }
}

export function subscribeCompanionApiStatusRefresh(
    listener: CompanionApiStatusRefreshListener
): () => void {
    statusRefreshListeners.add(listener);
    return () => {
        statusRefreshListeners.delete(listener);
    };
}

function presentStartFailure(failure: CompanionApiStartFailedPayload): void {
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
            ? 'view.settings.integrations.companion_api.port_in_use'
            : 'view.settings.integrations.companion_api.bind_failed';
    const reason = i18n.t(reasonKey, { port: failure.port });
    toast.error(
        i18n.t('view.settings.integrations.companion_api.start_failed', {
            reason
        })
    );
}
