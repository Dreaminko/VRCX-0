import {
    ArchiveRestoreIcon,
    CircleCheckIcon,
    LoaderCircleIcon
} from 'lucide-react';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { formatDateTime } from '@/lib/dateTime';
import {
    profileBackupErrorKey,
    profileRestoreFailureKey
} from '@/services/profileBackupI18n';
import {
    requestProfileRestore,
    type ProfileRestoreValidation
} from '@/services/profileBackupService';
import { useProfileBackupStore } from '@/state/profileBackupStore';
import {
    AlertDialog,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle
} from '@/ui/shadcn/alert-dialog';
import { Button } from '@/ui/shadcn/button';

function RestoreMetadata({
    validation
}: {
    validation: ProfileRestoreValidation;
}) {
    const { t } = useTranslation();
    const { manifest } = validation;
    const rows = [
        [
            t('profile_backup.created_at'),
            formatDateTime(manifest.createdAt, {
                dateStyle: 'medium',
                timeStyle: 'short'
            })
        ],
        [t('profile_backup.app_version'), manifest.appVersion],
        [t('profile_backup.database_version'), String(manifest.dbVersion)],
        [t('profile_backup.source_file'), validation.sourceFileName]
    ];

    return (
        <div className="space-y-4">
            <div className="flex items-center gap-3 rounded-lg border border-emerald-500/25 bg-emerald-500/[0.06] px-3 py-2.5 text-emerald-800 dark:text-emerald-200">
                <CircleCheckIcon className="size-5 shrink-0" />
                <div>
                    <div className="text-sm font-medium">
                        {t('profile_backup.backup_verified')}
                    </div>
                    <div className="mt-0.5 text-xs text-emerald-800/70 dark:text-emerald-100/65">
                        {t('profile_backup.checks_passed')}
                    </div>
                </div>
            </div>
            <dl className="bg-muted/20 grid gap-x-5 gap-y-3 rounded-lg border p-4 text-sm sm:grid-cols-2">
                {rows.map(([label, value]) => (
                    <div key={label} className="min-w-0 space-y-1">
                        <dt className="text-muted-foreground text-xs">
                            {label}
                        </dt>
                        <dd className="min-w-0 font-medium break-all">
                            {value || '-'}
                        </dd>
                    </div>
                ))}
            </dl>
        </div>
    );
}

export function ProfileBackupDialogs() {
    const { t } = useTranslation();
    const status = useProfileBackupStore((state) => state.status);
    const claimOutcomeNotification = useProfileBackupStore(
        (state) => state.claimOutcomeNotification
    );
    const restoreDialog = useProfileBackupStore((state) => state.restoreDialog);
    const restoreRequesting = useProfileBackupStore(
        (state) => state.restoreRequesting
    );
    const closeRestoreDialog = useProfileBackupStore(
        (state) => state.closeRestoreDialog
    );
    const setRestoreRequesting = useProfileBackupStore(
        (state) => state.setRestoreRequesting
    );
    const restartingText = t('profile_backup.restarting_to_restore');

    useEffect(() => {
        const outcome = status.lastOutcome;
        if (!outcome || !claimOutcomeNotification(outcome.revision)) {
            return;
        }
        if (outcome.succeeded) {
            toast.success(t('profile_backup.backup_saved'), {
                description: outcome.fileName || undefined
            });
            return;
        }
        toast.error(
            t(
                outcome.errorCode
                    ? profileBackupErrorKey(outcome.errorCode)
                    : 'profile_backup.error.unknown'
            )
        );
    }, [claimOutcomeNotification, status.lastOutcome, t]);

    async function confirmRestore() {
        if (!restoreDialog || restoreRequesting) {
            return;
        }
        setRestoreRequesting(true);
        try {
            const outcome = await requestProfileRestore(
                restoreDialog.path,
                restoreDialog.validation.stagedSha256
            );
            if (!outcome.validation) {
                setRestoreRequesting(false);
                toast.error(
                    t(
                        outcome.failure
                            ? profileRestoreFailureKey(outcome.failure.code)
                            : 'profile_backup.error.unknown'
                    )
                );
            }
        } catch {
            setRestoreRequesting(false);
            toast.error(t('profile_backup.restore_request_failed'));
        }
    }

    return (
        <AlertDialog
            open={Boolean(restoreDialog)}
            onOpenChange={(open) => {
                if (!open && !restoreRequesting) {
                    closeRestoreDialog();
                }
            }}
        >
            <AlertDialogContent className="sm:max-w-lg">
                <AlertDialogHeader>
                    <div className="flex items-start gap-3">
                        <div className="bg-destructive/10 text-destructive flex size-10 shrink-0 items-center justify-center rounded-xl">
                            <ArchiveRestoreIcon className="size-5" />
                        </div>
                        <div className="min-w-0 space-y-2">
                            <AlertDialogTitle>
                                {t('profile_backup.restore_confirm_title')}
                            </AlertDialogTitle>
                            <AlertDialogDescription>
                                {restoreRequesting
                                    ? restartingText
                                    : t(
                                          'profile_backup.restore_confirm_description'
                                      )}
                            </AlertDialogDescription>
                        </div>
                    </div>
                </AlertDialogHeader>
                {restoreDialog ? (
                    <RestoreMetadata validation={restoreDialog.validation} />
                ) : null}
                <AlertDialogFooter>
                    <Button
                        type="button"
                        variant="outline"
                        disabled={restoreRequesting}
                        onClick={closeRestoreDialog}
                    >
                        {t('common.actions.cancel')}
                    </Button>
                    <Button
                        type="button"
                        variant="destructive"
                        disabled={restoreRequesting}
                        onClick={() => {
                            void confirmRestore();
                        }}
                    >
                        {restoreRequesting ? (
                            <LoaderCircleIcon
                                className="animate-spin motion-reduce:animate-none"
                                data-icon="inline-start"
                            />
                        ) : (
                            <ArchiveRestoreIcon data-icon="inline-start" />
                        )}
                        {restoreRequesting
                            ? restartingText
                            : t('profile_backup.restore_and_restart')}
                    </Button>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}
