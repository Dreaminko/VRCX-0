import { describe, expect, it, vi } from 'vitest';

const runtimeState = vi.hoisted(() => ({
    commands: {
        appModerationSyncRefresh: vi.fn(),
        appModerationSyncUpdate: vi.fn()
    }
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: runtimeState.commands
}));

describe('moderationSyncService', () => {
    it('preserves a typed missing-credentials refresh error', async () => {
        const error = Object.assign(new Error('Missing Credentials'), {
            code: 'vrchat_api',
            statusCode: 401
        });
        runtimeState.commands.appModerationSyncRefresh.mockRejectedValueOnce(
            error
        );
        const { refreshModerationSync } =
            await import('./moderationSyncService');

        await expect(
            refreshModerationSync({ userId: 'usr_current', endpoint: '' })
        ).rejects.toBe(error);
    });

    it('preserves a typed missing-credentials mutation error', async () => {
        const error = Object.assign(new Error('Missing Credentials'), {
            code: 'vrchat_api',
            statusCode: 401
        });
        runtimeState.commands.appModerationSyncUpdate.mockRejectedValueOnce(
            error
        );
        const { updateModerationSync } =
            await import('./moderationSyncService');

        await expect(
            updateModerationSync({
                targetUserId: 'usr_target',
                type: 'block',
                enabled: false
            })
        ).rejects.toBe(error);
    });
});
