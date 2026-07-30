import { commands } from '@/platform/tauri/bindings';
import { normalizeString } from '@/shared/utils/string';
import { useRuntimeStore } from '@/state/runtimeStore';

import { isHostCapabilityAvailable } from './hostCapabilityService';

let lastRuntimeStateSignature: string | null = null;

function isRuntimeGameClientLifecycleActive(): boolean {
    return isHostCapabilityAvailable('runtimeGameClientLifecycle');
}

function getRuntimeLocationMirror(): string {
    const runtimeState = useRuntimeStore.getState();
    return (
        normalizeString(runtimeState.gameState.currentLocation) ||
        normalizeString(runtimeState.auth.currentUserSnapshot?.location)
    );
}

export async function syncRuntimeGameClientState(): Promise<void> {
    if (!isRuntimeGameClientLifecycleActive()) {
        lastRuntimeStateSignature = null;
        return;
    }

    const currentLocation = getRuntimeLocationMirror();
    if (currentLocation === lastRuntimeStateSignature) {
        return;
    }
    lastRuntimeStateSignature = currentLocation;

    try {
        await commands.appSetGameClientRuntimeState(currentLocation);
    } catch (error) {
        lastRuntimeStateSignature = null;
        console.warn('Failed to sync game client runtime state:', error);
    }
}

export function startRuntimeGameClientSync(): () => void {
    const sync = () => {
        syncRuntimeGameClientState();
    };
    const unsubscribeRuntime = useRuntimeStore.subscribe(sync);
    sync();

    return () => {
        unsubscribeRuntime();
    };
}
