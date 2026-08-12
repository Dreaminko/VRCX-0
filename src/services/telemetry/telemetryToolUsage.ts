import { TELEMETRY_TOOL_KEYS } from './telemetryContract';
import { recordTelemetryEvent } from './telemetryEvent';

const telemetryToolKeys = new Set<string>(TELEMETRY_TOOL_KEYS);

export function recordToolOpen(tool: string): void {
    if (!telemetryToolKeys.has(tool)) {
        return;
    }
    recordTelemetryEvent({ type: 'toolOpen', tool });
}
