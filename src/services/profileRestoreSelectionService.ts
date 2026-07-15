import { toast } from 'sonner';

import { profileRestoreFailureKey } from '@/services/profileBackupI18n';
import { validateProfileRestore } from '@/services/profileBackupService';
import { openFileSelectorDialog } from '@/services/shellIntegrationService';
import { useProfileBackupStore } from '@/state/profileBackupStore';

import i18n from './i18nService';

let selectionPending = false;

export async function selectProfileBackupToRestore(
    defaultPath: string = ''
): Promise<boolean> {
    if (selectionPending) {
        return false;
    }

    selectionPending = true;
    try {
        let path = '';
        try {
            path = await openFileSelectorDialog(
                defaultPath,
                '.vrcx0backup',
                `${i18n.t('profile_backup.file_filter')} (*.vrcx0backup)|*.vrcx0backup`
            );
        } catch {
            toast.error(i18n.t('profile_backup.file_selection_failed'));
            return false;
        }

        if (!path) {
            return false;
        }

        const toastId = toast.loading(
            i18n.t('profile_backup.validating_restore')
        );
        try {
            const outcome = await validateProfileRestore(path);
            if (!outcome.validation) {
                toast.error(
                    i18n.t(
                        outcome.failure
                            ? profileRestoreFailureKey(outcome.failure.code)
                            : 'profile_backup.error.unknown'
                    )
                );
                return false;
            }
            useProfileBackupStore
                .getState()
                .openRestoreDialog(path, outcome.validation);
            return true;
        } catch {
            toast.error(i18n.t('profile_backup.restore_validation_failed'));
            return false;
        } finally {
            toast.dismiss(toastId);
        }
    } finally {
        selectionPending = false;
    }
}
