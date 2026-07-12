import { commands } from '@/platform/tauri/bindings';
import { useSessionStore } from '@/state/sessionStore';

import { APP_UPDATE_CHECK_INTERVAL_SECONDS } from './backgroundMaintenanceTiming';
import { checkForAppUpdate } from './backgroundMaintenanceUpdateService';
import {
    recordRuntimeJobTelemetry,
    runRuntimeTelemetryJob
} from './runtimeJobTelemetryService';

let running = false;

async function getDueRuntimeScheduledFrontendJobs() {
    const dueJobs = await commands
        .appRuntimeFrontendScheduleDueJobsGet()
        .catch((error: unknown): unknown[] => {
            console.warn('Failed to read runtime maintenance due jobs:', error);
            return [];
        });
    return new Set(Array.isArray(dueJobs) ? dueJobs : []);
}

export async function runBackgroundMaintenanceTick() {
    if (running || !useSessionStore.getState().isLoggedIn) {
        return;
    }

    running = true;
    const dueJobs = await getDueRuntimeScheduledFrontendJobs();
    const hasDueJobs = dueJobs.size > 0;
    if (hasDueJobs) {
        recordRuntimeJobTelemetry({
            name: 'backgroundMaintenanceTick',
            owner: 'frontend',
            status: 'running',
            detail: 'Frontend executor is running Rust-scheduled maintenance.'
        });
    }

    try {
        if (dueJobs.has('appUpdateCheck')) {
            await runRuntimeTelemetryJob(
                {
                    name: 'appUpdateCheck',
                    cadenceSeconds: APP_UPDATE_CHECK_INTERVAL_SECONDS,
                    detail: 'Running Rust-scheduled frontend maintenance task appUpdateCheck.'
                },
                checkForAppUpdate
            );
        }
    } finally {
        running = false;
        if (hasDueJobs) {
            recordRuntimeJobTelemetry({
                name: 'backgroundMaintenanceTick',
                owner: 'frontend',
                status: 'completed',
                detail: 'Rust-scheduled frontend maintenance tick completed.'
            });
        }
    }
}

export function resetBackgroundMaintenance() {
    commands
        .appRuntimeFrontendScheduleSchedulesReset()
        .catch((error: unknown) => {
            console.warn(
                'Failed to reset runtime maintenance scheduler:',
                error
            );
        });
}
