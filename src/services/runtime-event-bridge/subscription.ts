import { tauriClient } from '@/platform/tauri/client';

import type { RuntimeEventName, RuntimeEventPayloadMap } from './types';

export function subscribeRuntimeEvent<Name extends RuntimeEventName>(
    name: Name,
    handler: (payload: RuntimeEventPayloadMap[Name]) => void
): Promise<() => void> {
    return tauriClient.events.subscribe(name, handler);
}
