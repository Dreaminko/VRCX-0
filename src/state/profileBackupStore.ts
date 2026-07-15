import { create } from 'zustand';

import type {
    ProfileBackupStatus,
    ProfileRestoreResult,
    ProfileRestoreRollbackState,
    ProfileRestoreValidation
} from '@/services/profileBackupService';

const NOTIFIED_OUTCOME_REVISION_KEY =
    'vrcx-0.profile-backup.notified-outcome-revision';

function readNotifiedOutcomeRevision(): number {
    if (typeof window === 'undefined') {
        return -1;
    }
    try {
        const revision = Number.parseInt(
            window.sessionStorage.getItem(NOTIFIED_OUTCOME_REVISION_KEY) || '',
            10
        );
        return Number.isSafeInteger(revision) && revision >= 0 ? revision : -1;
    } catch {
        return -1;
    }
}

function writeNotifiedOutcomeRevision(revision: number) {
    if (typeof window === 'undefined') {
        return;
    }
    try {
        window.sessionStorage.setItem(
            NOTIFIED_OUTCOME_REVISION_KEY,
            String(revision)
        );
    } catch {
        return;
    }
}

type ProfileRestoreDialogState = {
    path: string;
    validation: ProfileRestoreValidation;
};

type ProfileBackupStore = {
    status: ProfileBackupStatus;
    lastAppliedRevision: number;
    lastNotifiedOutcomeRevision: number;
    restoreDialog: ProfileRestoreDialogState | null;
    restoreRequesting: boolean;
    startupRestoreResult: ProfileRestoreResult | null;
    startupRestoreResultChecked: boolean;
    restoreRollbackState: ProfileRestoreRollbackState | null;
    restoreRollbackStateRequestRevision: number;
    restoreRollbackCleanupRunning: boolean;
    applyStatus(status: ProfileBackupStatus): void;
    claimOutcomeNotification(revision: number): boolean;
    openRestoreDialog(path: string, validation: ProfileRestoreValidation): void;
    closeRestoreDialog(): void;
    setRestoreRequesting(requesting: boolean): void;
    beginStartupRestoreResultCheck(): boolean;
    setStartupRestoreResult(result: ProfileRestoreResult | null): void;
    clearStartupRestoreResult(): void;
    beginRestoreRollbackStateRefresh(): number;
    completeRestoreRollbackStateRefresh(
        revision: number,
        state: ProfileRestoreRollbackState | null
    ): boolean;
    setRestoreRollbackState(state: ProfileRestoreRollbackState): void;
    beginRestoreRollbackCleanup(): boolean;
    finishRestoreRollbackCleanup(): void;
    resetProfileBackupState(): void;
};

function createIdleStatus(): ProfileBackupStatus {
    return {
        revision: 0,
        state: 'idle',
        kind: null,
        phase: null,
        percent: null,
        error: null,
        lastOutcome: null
    };
}

export const useProfileBackupStore = create<ProfileBackupStore>((set, get) => ({
    status: createIdleStatus(),
    lastAppliedRevision: -1,
    lastNotifiedOutcomeRevision: readNotifiedOutcomeRevision(),
    restoreDialog: null,
    restoreRequesting: false,
    startupRestoreResult: null,
    startupRestoreResultChecked: false,
    restoreRollbackState: null,
    restoreRollbackStateRequestRevision: 0,
    restoreRollbackCleanupRunning: false,
    applyStatus(status) {
        set((current) => {
            if (status.revision <= current.lastAppliedRevision) {
                return current;
            }
            return {
                status,
                lastAppliedRevision: status.revision
            };
        });
    },
    claimOutcomeNotification(revision) {
        if (revision <= get().lastNotifiedOutcomeRevision) {
            return false;
        }
        writeNotifiedOutcomeRevision(revision);
        set({ lastNotifiedOutcomeRevision: revision });
        return true;
    },
    openRestoreDialog(path, validation) {
        set({
            restoreDialog: { path, validation },
            restoreRequesting: false
        });
    },
    closeRestoreDialog() {
        set({ restoreDialog: null, restoreRequesting: false });
    },
    setRestoreRequesting(restoreRequesting) {
        set({ restoreRequesting });
    },
    beginStartupRestoreResultCheck() {
        if (get().startupRestoreResultChecked) {
            return false;
        }
        set({ startupRestoreResultChecked: true });
        return true;
    },
    setStartupRestoreResult(startupRestoreResult) {
        set({ startupRestoreResult });
    },
    clearStartupRestoreResult() {
        set({ startupRestoreResult: null });
    },
    beginRestoreRollbackStateRefresh() {
        const revision = get().restoreRollbackStateRequestRevision + 1;
        set({
            restoreRollbackState: null,
            restoreRollbackStateRequestRevision: revision
        });
        return revision;
    },
    completeRestoreRollbackStateRefresh(revision, restoreRollbackState) {
        if (revision !== get().restoreRollbackStateRequestRevision) {
            return false;
        }
        set({ restoreRollbackState });
        return true;
    },
    setRestoreRollbackState(restoreRollbackState) {
        set((state) => ({
            restoreRollbackState,
            restoreRollbackStateRequestRevision:
                state.restoreRollbackStateRequestRevision + 1
        }));
    },
    beginRestoreRollbackCleanup() {
        if (get().restoreRollbackCleanupRunning) {
            return false;
        }
        set({ restoreRollbackCleanupRunning: true });
        return true;
    },
    finishRestoreRollbackCleanup() {
        set({ restoreRollbackCleanupRunning: false });
    },
    resetProfileBackupState() {
        set((state) => ({
            status: createIdleStatus(),
            lastAppliedRevision: -1,
            lastNotifiedOutcomeRevision: readNotifiedOutcomeRevision(),
            restoreDialog: null,
            restoreRequesting: false,
            startupRestoreResult: null,
            startupRestoreResultChecked: false,
            restoreRollbackState: null,
            restoreRollbackStateRequestRevision:
                state.restoreRollbackStateRequestRevision + 1,
            restoreRollbackCleanupRunning: false
        }));
    }
}));
