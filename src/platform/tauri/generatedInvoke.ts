import { recordErrorLog } from '../../services/errorLogService';
import { notifySQLiteError } from '../../shared/sqliteErrorEvents';
import { normalizePlatformError } from './errors';
import { invokeTauri } from './invoke';

export async function invoke<TReturn = unknown>(
    command: string,
    args?: Record<string, unknown>
): Promise<TReturn> {
    try {
        return await invokeTauri<TReturn>(command, args);
    } catch (error) {
        const normalizedError = normalizePlatformError(
            error,
            `Tauri command failed: ${command}`
        );

        if (command !== 'app__append_error_log') {
            recordErrorLog('rust:command', [
                `command: ${command}`,
                normalizedError
            ]);
        }
        notifySQLiteError(normalizedError);

        throw normalizedError;
    }
}
