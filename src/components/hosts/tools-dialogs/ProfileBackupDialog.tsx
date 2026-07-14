import {
    CalendarClockIcon,
    DatabaseBackupIcon,
    FolderOpenIcon,
    LoaderCircleIcon,
    RotateCcwIcon,
    SaveIcon,
    ShieldAlertIcon,
    VaultIcon
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { useProfileBackupSettings } from '@/features/tools/useProfileBackupSettings';
import { formatDateTime } from '@/lib/dateTime';
import {
    profileBackupErrorKey,
    profileBackupPhaseKey
} from '@/services/profileBackupI18n';
import {
    discardPendingProfileBackup,
    dismissProfileBackupError,
    retryProfileBackupDelivery,
    type ProfileBackupStatus
} from '@/services/profileBackupService';
import { useProfileBackupStore } from '@/state/profileBackupStore';
import { Alert, AlertDescription, AlertTitle } from '@/ui/shadcn/alert';
import { Button } from '@/ui/shadcn/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle
} from '@/ui/shadcn/dialog';
import { Input } from '@/ui/shadcn/input';
import { Progress } from '@/ui/shadcn/progress';
import { Switch } from '@/ui/shadcn/switch';

type ProfileBackupDialogProps = {
    open: boolean;
    onOpenChange: (open: boolean) => void;
};

function getStatusTitleKey(status: ProfileBackupStatus): string {
    switch (status.state) {
        case 'running':
            return status.kind === 'auto'
                ? 'profile_backup.automatic_running'
                : 'profile_backup.manual_running';
        case 'retryable':
            return 'profile_backup.retryable_title';
        case 'error':
            return 'profile_backup.error_title';
        case 'idle':
            return '';
    }
}

function BackupPath({ value }: { value: string }) {
    const { t } = useTranslation();
    return (
        <div
            className="bg-background/60 text-muted-foreground flex min-w-0 flex-1 items-center gap-2 rounded-lg border px-3 py-2 text-xs"
            title={value || undefined}
        >
            <FolderOpenIcon className="size-3.5 shrink-0" />
            <span className="truncate">
                {value || t('profile_backup.location_not_set')}
            </span>
        </div>
    );
}

export function ProfileBackupDialog({
    open,
    onOpenChange
}: ProfileBackupDialogProps) {
    const { t } = useTranslation();
    const status = useProfileBackupStore((state) => state.status);
    const applyStatus = useProfileBackupStore((state) => state.applyStatus);
    const {
        settings,
        loading,
        saving,
        startingManualBackup,
        validatingRestore,
        numericDraftValue,
        setNumericDraft,
        commitNumericDraft,
        setAutoEnabled,
        chooseAutomaticBackupFolder,
        startManualBackup,
        selectBackupToRestore
    } = useProfileBackupSettings(open);
    const [statusActionRunning, setStatusActionRunning] = useState(false);
    const isRunning = status.state === 'running';
    const automaticEnabled = Boolean(settings?.autoEnabled);
    const disabled = loading || saving || !settings || isRunning;
    const statusErrorKey = status.error
        ? profileBackupErrorKey(status.error.code)
        : 'profile_backup.error.unknown';
    const lastAutomaticBackup = settings?.lastAutoAt
        ? formatDateTime(settings.lastAutoAt, {
              dateStyle: 'medium',
              timeStyle: 'short'
          })
        : t('profile_backup.no_automatic_backup');
    const statusTitleKey = getStatusTitleKey(status);
    const runningPhaseKey = profileBackupPhaseKey(status);
    const runningPhaseLabel = t(runningPhaseKey);

    async function runStatusAction(action: 'retry' | 'discard' | 'dismiss') {
        setStatusActionRunning(true);
        try {
            if (action === 'retry') {
                const outcome = await retryProfileBackupDelivery();
                applyStatus(outcome.status);
                if (!outcome.accepted) {
                    toast.error(
                        t(
                            outcome.error
                                ? profileBackupErrorKey(outcome.error.code)
                                : 'profile_backup.error.unknown'
                        )
                    );
                }
            } else if (action === 'discard') {
                const outcome = await discardPendingProfileBackup();
                applyStatus(outcome.status);
                if (!outcome.accepted) {
                    toast.error(
                        t(
                            outcome.error
                                ? profileBackupErrorKey(outcome.error.code)
                                : 'profile_backup.error.unknown'
                        )
                    );
                }
            } else {
                applyStatus(await dismissProfileBackupError());
            }
        } catch {
            toast.error(t('profile_backup.action_failed'));
        } finally {
            setStatusActionRunning(false);
        }
    }

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="flex max-h-[85vh] min-h-0 flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl">
                <DialogHeader className="border-b px-6 py-5">
                    <DialogTitle>{t('profile_backup.header')}</DialogTitle>
                    <DialogDescription>
                        {t('profile_backup.tools_description')}
                    </DialogDescription>
                </DialogHeader>

                <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
                    <Alert className="border-amber-500/25 bg-amber-500/[0.06] text-amber-950 dark:text-amber-100">
                        <ShieldAlertIcon className="text-amber-600 dark:text-amber-400" />
                        <AlertTitle>
                            {t('profile_backup.unencrypted_warning_title')}
                        </AlertTitle>
                        <AlertDescription className="text-amber-900/75 dark:text-amber-100/70">
                            {t('profile_backup.unencrypted_warning')}
                        </AlertDescription>
                    </Alert>

                    <section className="bg-card/70 overflow-hidden rounded-xl border shadow-sm">
                        <div className="flex items-start gap-4 p-4 sm:items-center">
                            <div className="bg-primary/10 text-primary border-primary/15 flex size-10 shrink-0 items-center justify-center rounded-xl border">
                                <VaultIcon className="size-5" />
                            </div>
                            <div className="min-w-0 flex-1">
                                <div className="flex flex-wrap items-center gap-2">
                                    <h3 className="font-heading text-sm font-medium">
                                        {t('profile_backup.automatic')}
                                    </h3>
                                    <span
                                        className={
                                            automaticEnabled
                                                ? 'bg-primary/10 text-primary rounded-full px-2 py-0.5 text-[0.7rem] font-medium'
                                                : 'bg-muted text-muted-foreground rounded-full px-2 py-0.5 text-[0.7rem] font-medium'
                                        }
                                    >
                                        {t(
                                            automaticEnabled
                                                ? 'profile_backup.automatic_on'
                                                : 'profile_backup.automatic_off'
                                        )}
                                    </span>
                                </div>
                                <p className="text-muted-foreground mt-1 text-xs leading-relaxed">
                                    {t('profile_backup.automatic_description')}
                                </p>
                            </div>
                            <Switch
                                checked={automaticEnabled}
                                disabled={disabled}
                                aria-label={t(
                                    'profile_backup.enable_automatic'
                                )}
                                onCheckedChange={(checked) => {
                                    void setAutoEnabled(checked);
                                }}
                            />
                        </div>

                        {status.state !== 'idle' ? (
                            <div
                                className={
                                    isRunning
                                        ? 'border-primary/15 bg-primary/[0.04] border-y px-4 py-3'
                                        : 'border-destructive/20 bg-destructive/[0.04] border-y px-4 py-3'
                                }
                            >
                                <div className="flex items-center justify-between gap-3">
                                    <div className="text-sm font-medium">
                                        {t(statusTitleKey)}
                                    </div>
                                    {isRunning && status.percent !== null ? (
                                        <span className="text-muted-foreground text-xs tabular-nums">
                                            {`${status.percent}%`}
                                        </span>
                                    ) : null}
                                </div>
                                {isRunning ? (
                                    <div className="space-y-2 pt-2">
                                        <div
                                            role="status"
                                            aria-live="polite"
                                            className="text-muted-foreground flex items-center gap-1.5 text-xs"
                                        >
                                            {status.percent === null ? (
                                                <LoaderCircleIcon
                                                    aria-hidden="true"
                                                    className="size-3 animate-spin motion-reduce:animate-none"
                                                />
                                            ) : null}
                                            {runningPhaseLabel}
                                        </div>
                                        {status.percent !== null ? (
                                            <Progress
                                                aria-label={runningPhaseLabel}
                                                value={status.percent}
                                            />
                                        ) : null}
                                    </div>
                                ) : (
                                    <div className="space-y-3 pt-2">
                                        <p className="text-muted-foreground text-xs leading-relaxed">
                                            {t(statusErrorKey)}
                                        </p>
                                        {status.error?.path ? (
                                            <div className="bg-background/60 rounded-lg border p-2 text-xs break-all">
                                                {status.error.path}
                                            </div>
                                        ) : null}
                                        <div className="flex flex-wrap justify-end gap-2">
                                            {status.state === 'retryable' ? (
                                                <>
                                                    <Button
                                                        type="button"
                                                        size="sm"
                                                        variant="outline"
                                                        disabled={
                                                            statusActionRunning
                                                        }
                                                        onClick={() => {
                                                            void runStatusAction(
                                                                'discard'
                                                            );
                                                        }}
                                                    >
                                                        {t(
                                                            'profile_backup.discard_backup'
                                                        )}
                                                    </Button>
                                                    <Button
                                                        type="button"
                                                        size="sm"
                                                        disabled={
                                                            statusActionRunning
                                                        }
                                                        onClick={() => {
                                                            void runStatusAction(
                                                                'retry'
                                                            );
                                                        }}
                                                    >
                                                        {t(
                                                            'profile_backup.retry_save'
                                                        )}
                                                    </Button>
                                                </>
                                            ) : (
                                                <Button
                                                    type="button"
                                                    size="sm"
                                                    disabled={
                                                        statusActionRunning
                                                    }
                                                    onClick={() => {
                                                        void runStatusAction(
                                                            'dismiss'
                                                        );
                                                    }}
                                                >
                                                    {t(
                                                        'profile_backup.dismiss_error'
                                                    )}
                                                </Button>
                                            )}
                                        </div>
                                    </div>
                                )}
                            </div>
                        ) : null}

                        <div className="space-y-4 p-4">
                            <div className="flex items-center gap-2 text-xs">
                                <CalendarClockIcon className="text-muted-foreground size-4" />
                                <span className="text-muted-foreground">
                                    {t('profile_backup.last_automatic_backup')}
                                </span>
                                <span className="font-medium">
                                    {lastAutomaticBackup}
                                </span>
                            </div>

                            <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                                <BackupPath
                                    value={settings?.autoTargetDir || ''}
                                />
                                <Button
                                    type="button"
                                    variant="outline"
                                    disabled={disabled}
                                    onClick={() => {
                                        void chooseAutomaticBackupFolder();
                                    }}
                                >
                                    <FolderOpenIcon data-icon="inline-start" />
                                    {t('profile_backup.change_folder')}
                                </Button>
                            </div>

                            <div className="grid gap-3 sm:grid-cols-2">
                                <label className="bg-background/40 flex items-center justify-between gap-3 rounded-lg border px-3 py-2.5">
                                    <span className="text-sm font-medium">
                                        {t('profile_backup.interval')}
                                    </span>
                                    <span className="flex items-center gap-2">
                                        <Input
                                            type="number"
                                            min={1}
                                            max={30}
                                            disabled={disabled}
                                            value={numericDraftValue(
                                                'autoIntervalDays'
                                            )}
                                            onChange={(event) =>
                                                setNumericDraft(
                                                    'autoIntervalDays',
                                                    event.currentTarget.value
                                                )
                                            }
                                            onBlur={() => {
                                                void commitNumericDraft(
                                                    'autoIntervalDays'
                                                );
                                            }}
                                            className="w-20"
                                            aria-label={t(
                                                'profile_backup.interval'
                                            )}
                                        />
                                        <span className="text-muted-foreground text-xs">
                                            {t('profile_backup.days')}
                                        </span>
                                    </span>
                                </label>
                                <label className="bg-background/40 flex items-center justify-between gap-3 rounded-lg border px-3 py-2.5">
                                    <span className="text-sm font-medium">
                                        {t('profile_backup.keep_count')}
                                    </span>
                                    <Input
                                        type="number"
                                        min={2}
                                        max={6}
                                        disabled={disabled}
                                        value={numericDraftValue(
                                            'autoRetainExtra'
                                        )}
                                        onChange={(event) =>
                                            setNumericDraft(
                                                'autoRetainExtra',
                                                event.currentTarget.value
                                            )
                                        }
                                        onBlur={() => {
                                            void commitNumericDraft(
                                                'autoRetainExtra'
                                            );
                                        }}
                                        className="w-20"
                                        aria-label={t(
                                            'profile_backup.keep_count'
                                        )}
                                    />
                                </label>
                            </div>
                        </div>
                    </section>

                    <section className="flex flex-col gap-4 px-1 sm:flex-row sm:items-center">
                        <div className="bg-primary/10 text-primary flex size-10 shrink-0 items-center justify-center rounded-xl">
                            <DatabaseBackupIcon className="size-5" />
                        </div>
                        <div className="min-w-0 flex-1">
                            <h3 className="font-heading text-sm font-medium">
                                {t('profile_backup.manual_backup')}
                            </h3>
                            <p className="text-muted-foreground mt-1 text-xs leading-relaxed">
                                {t('profile_backup.manual_backup_description')}
                            </p>
                        </div>
                        <div className="sm:self-center">
                            <Button
                                type="button"
                                disabled={disabled || startingManualBackup}
                                onClick={() => {
                                    void startManualBackup();
                                }}
                            >
                                <SaveIcon data-icon="inline-start" />
                                {t('profile_backup.backup_now')}
                            </Button>
                        </div>
                    </section>

                    <section className="bg-card/40 relative overflow-hidden rounded-xl border border-amber-500/25 p-4">
                        <div className="flex flex-col gap-4 sm:flex-row sm:items-center">
                            <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-amber-500/10 text-amber-700 dark:text-amber-300">
                                <RotateCcwIcon className="size-5" />
                            </div>
                            <div className="min-w-0 flex-1">
                                <h3 className="font-heading text-sm font-medium">
                                    {t('profile_backup.restore')}
                                </h3>
                                <p className="text-muted-foreground mt-1 text-xs leading-relaxed">
                                    {t('profile_backup.restore_description')}
                                </p>
                            </div>
                            <div className="sm:self-center">
                                <Button
                                    type="button"
                                    variant="outline"
                                    disabled={disabled || validatingRestore}
                                    onClick={() => {
                                        void selectBackupToRestore();
                                    }}
                                >
                                    <RotateCcwIcon data-icon="inline-start" />
                                    {t('profile_backup.restore_from_backup')}
                                </Button>
                            </div>
                        </div>
                    </section>
                </div>
            </DialogContent>
        </Dialog>
    );
}
