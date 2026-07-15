import { describe, expect, it } from 'vitest';

import {
    profileBackupPhaseKey,
    profileRestoreRollbackErrorKey
} from './profileBackupI18n';
import type {
    ProfileBackupPhase,
    ProfileBackupStatus
} from './profileBackupService';

function runningStatus(
    phase: ProfileBackupPhase,
    percent: number | null
): ProfileBackupStatus {
    return {
        revision: 1,
        state: 'running',
        kind: 'manual',
        phase,
        percent,
        error: null,
        lastOutcome: null
    };
}

describe('profileBackupPhaseKey', () => {
    it('uses the regular delivery label while bytes are being copied', () => {
        expect(profileBackupPhaseKey(runningStatus('deliver', 42))).toBe(
            'profile_backup.phase_deliver'
        );
    });

    it('uses the finalizing label after delivery bytes are copied', () => {
        expect(profileBackupPhaseKey(runningStatus('deliver', null))).toBe(
            'profile_backup.phase_finalize'
        );
    });

    it('keeps an indeterminate snapshot in the preparation phase', () => {
        expect(profileBackupPhaseKey(runningStatus('snapshot', null))).toBe(
            'profile_backup.phase_snapshot'
        );
    });
});

describe('profileRestoreRollbackErrorKey', () => {
    it.each([
        ['operationBusy', 'profile_backup.rollback_error.busy'],
        ['pendingRestore', 'profile_backup.rollback_error.protected'],
        ['io', 'profile_backup.rollback_error.io']
    ] as const)('maps %s to a rollback-specific message', (code, key) => {
        expect(profileRestoreRollbackErrorKey(code)).toBe(key);
    });
});
