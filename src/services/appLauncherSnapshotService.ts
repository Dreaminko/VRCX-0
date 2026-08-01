import type {
    AppLauncherSnapshot,
    AppLauncherSnapshotEvent
} from '@/platform/tauri/bindings';
import appLauncherRepository from '@/repositories/appLauncherRepository';

type AppLauncherSnapshotListener = (snapshot: AppLauncherSnapshot) => void;

const listeners = new Set<AppLauncherSnapshotListener>();
let snapshotSequence = 0;
let latestSnapshot: AppLauncherSnapshot | null = null;

function publishSnapshot(snapshot: AppLauncherSnapshot) {
    latestSnapshot = snapshot;
    for (const listener of listeners) {
        listener(snapshot);
    }
    return snapshot;
}

export function handleAppLauncherSnapshotEvent(
    event: AppLauncherSnapshotEvent
): void {
    snapshotSequence += 1;
    publishSnapshot(event.snapshot);
}

export function subscribeAppLauncherSnapshot(
    listener: AppLauncherSnapshotListener
): () => void {
    listeners.add(listener);
    if (latestSnapshot) {
        listener(latestSnapshot);
    }
    return () => {
        listeners.delete(listener);
    };
}

export async function getCurrentAppLauncherSnapshot(): Promise<AppLauncherSnapshot> {
    snapshotSequence += 1;
    const requestSequence = snapshotSequence;
    const snapshot = await appLauncherRepository.snapshot();
    if (requestSequence !== snapshotSequence && latestSnapshot) {
        return latestSnapshot;
    }
    return publishSnapshot(snapshot);
}
