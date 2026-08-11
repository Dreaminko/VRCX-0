import vrchatAuthRepository from '@/repositories/vrchatAuthRepository';
import { useRuntimeStore } from '@/state/runtimeStore';
import {
    useVrchatConfigStore,
    type VrchatConfigSnapshot
} from '@/state/vrchatConfigStore';

function configSnapshot(value: unknown): VrchatConfigSnapshot {
    return value && typeof value === 'object' && !Array.isArray(value)
        ? (value as VrchatConfigSnapshot)
        : {};
}

export async function loadVrchatConfigSnapshot({
    force = false
}: {
    force?: boolean;
} = {}): Promise<VrchatConfigSnapshot | null> {
    const auth = useRuntimeStore.getState().auth;
    const expectedUserId = String(auth.currentUserId || '');
    const expectedEndpoint = String(auth.currentUserEndpoint || '');
    const response = force
        ? await vrchatAuthRepository.refreshConfig()
        : await vrchatAuthRepository.getConfig();
    const currentAuth = useRuntimeStore.getState().auth;
    if (
        String(currentAuth.currentUserId || '') !== expectedUserId ||
        String(currentAuth.currentUserEndpoint || '') !== expectedEndpoint
    ) {
        return null;
    }
    const snapshot = configSnapshot(response.json);
    useVrchatConfigStore.getState().setSnapshot(snapshot);
    return snapshot;
}

export function resetVrchatConfigSnapshot(): void {
    useVrchatConfigStore.getState().reset();
}
