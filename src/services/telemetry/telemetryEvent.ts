import { commands, type TelemetryClientEvent } from '@/platform/tauri/bindings';

export function recordTelemetryEvent(event: TelemetryClientEvent): void {
    void commands.appTelemetryRecordEvent(event).catch(() => {});
}

export async function submitTelemetryFeedback(content: string): Promise<void> {
    await commands.appTelemetrySubmitFeedback(content);
}
