// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest';

import type {
    ProfileBackupStatus,
    ProfileRestoreResult,
    ProfileRestoreRollbackState,
    ProfileRestoreValidation
} from '@/services/profileBackupService';

import { useProfileBackupStore } from './profileBackupStore';

function status(
    revision: number,
    state: ProfileBackupStatus['state'] = 'running'
): ProfileBackupStatus {
    return {
        revision,
        state,
        kind: state === 'idle' ? null : 'manual',
        phase: state === 'running' ? 'snapshot' : null,
        percent: state === 'running' ? 10 : null,
        error: null,
        lastOutcome: null
    };
}

describe('profileBackupStore', () => {
    beforeEach(() => {
        sessionStorage.clear();
        useProfileBackupStore.getState().resetProfileBackupState();
    });

    it('ignores status snapshots older than the latest runtime event', () => {
        useProfileBackupStore.getState().applyStatus(status(4));
        useProfileBackupStore.getState().applyStatus(status(3, 'idle'));

        expect(useProfileBackupStore.getState().status).toEqual(status(4));
        expect(useProfileBackupStore.getState().lastAppliedRevision).toBe(4);
    });

    it('keeps restore confirmation data outside the runtime status snapshot', () => {
        const validation: ProfileRestoreValidation = {
            sourceFileName: 'backup.vrcx0backup',
            stagedSha256: 'abc123',
            stagedBytes: 42,
            archive: 'valid',
            appVersion: 'compatible',
            databaseVersion: 'compatible',
            database: 'valid',
            manifest: {
                createdAt: '2026-07-14T07:30:00Z',
                appVersion: '2.13.0',
                dbVersion: 18,
                platform: 'windows',
                kind: 'manual'
            }
        };

        useProfileBackupStore
            .getState()
            .openRestoreDialog('D:\\backup.vrcx0backup', validation);

        expect(useProfileBackupStore.getState().restoreDialog).toEqual({
            path: 'D:\\backup.vrcx0backup',
            validation
        });
    });

    it('keeps the notified outcome revision across a WebView reload', async () => {
        expect(
            useProfileBackupStore.getState().claimOutcomeNotification(12)
        ).toBe(true);
        expect(
            useProfileBackupStore.getState().claimOutcomeNotification(12)
        ).toBe(false);

        vi.resetModules();
        const reloaded = await import('./profileBackupStore');

        expect(
            reloaded.useProfileBackupStore.getState()
                .lastNotifiedOutcomeRevision
        ).toBe(12);
    });

    it('runs the startup restore result check exactly once', () => {
        const result: ProfileRestoreResult = {
            status: 'failed',
            dataDisposition: 'rolledBack',
            sourceFileName: 'backup.vrcx0backup',
            failure: {
                code: 'databaseOpenFailed',
                path: null
            }
        };

        expect(
            useProfileBackupStore.getState().beginStartupRestoreResultCheck()
        ).toBe(true);
        expect(
            useProfileBackupStore.getState().beginStartupRestoreResultCheck()
        ).toBe(false);

        useProfileBackupStore.getState().setStartupRestoreResult(result);
        expect(useProfileBackupStore.getState().startupRestoreResult).toEqual(
            result
        );
        useProfileBackupStore.getState().clearStartupRestoreResult();
        expect(
            useProfileBackupStore.getState().startupRestoreResult
        ).toBeNull();
    });

    it('hides rollback state while refreshing and ignores stale responses', () => {
        const retained: ProfileRestoreRollbackState = {
            count: 1,
            cleanupAllowed: true
        };
        useProfileBackupStore.getState().setRestoreRollbackState(retained);

        const first = useProfileBackupStore
            .getState()
            .beginRestoreRollbackStateRefresh();
        expect(
            useProfileBackupStore.getState().restoreRollbackState
        ).toBeNull();
        const second = useProfileBackupStore
            .getState()
            .beginRestoreRollbackStateRefresh();

        expect(
            useProfileBackupStore
                .getState()
                .completeRestoreRollbackStateRefresh(first, retained)
        ).toBe(false);
        expect(
            useProfileBackupStore
                .getState()
                .completeRestoreRollbackStateRefresh(second, retained)
        ).toBe(true);
        expect(useProfileBackupStore.getState().restoreRollbackState).toEqual(
            retained
        );
    });

    it('allows only one rollback cleanup confirmation at a time', () => {
        expect(
            useProfileBackupStore.getState().beginRestoreRollbackCleanup()
        ).toBe(true);
        expect(
            useProfileBackupStore.getState().beginRestoreRollbackCleanup()
        ).toBe(false);

        useProfileBackupStore.getState().finishRestoreRollbackCleanup();

        expect(
            useProfileBackupStore.getState().beginRestoreRollbackCleanup()
        ).toBe(true);
    });

    it('prevents an in-flight refresh from overwriting cleanup state', () => {
        const refresh = useProfileBackupStore
            .getState()
            .beginRestoreRollbackStateRefresh();
        useProfileBackupStore.getState().setRestoreRollbackState({
            count: 0,
            cleanupAllowed: false
        });

        expect(
            useProfileBackupStore
                .getState()
                .completeRestoreRollbackStateRefresh(refresh, {
                    count: 1,
                    cleanupAllowed: true
                })
        ).toBe(false);
        expect(useProfileBackupStore.getState().restoreRollbackState).toEqual({
            count: 0,
            cleanupAllowed: false
        });
    });
});
