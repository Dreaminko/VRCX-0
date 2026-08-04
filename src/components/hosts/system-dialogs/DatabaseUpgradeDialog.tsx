import { LoaderCircleIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import type { DatabaseUpgradeStage } from '@/platform/tauri/bindings';
import {
    confirmLegacyDatabaseMigration,
    createDatabaseUpgradeGitHubIssue,
    openDatabaseUpgradeFailureLogFolder,
    retryDatabaseUpgrade,
    startFreshDatabaseAfterUpgradeFailure,
    skipLegacyDatabaseMigration
} from '@/services/databaseUpgradeService';
import { useRuntimeStore } from '@/state/runtimeStore';
import { Button } from '@/ui/shadcn/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle
} from '@/ui/shadcn/dialog';
import { Progress } from '@/ui/shadcn/progress';

const DATABASE_UPGRADE_STAGE_KEYS: Record<DatabaseUpgradeStage, string> = {
    preflight: 'message.database.upgrade_stage.preflight',
    prepareLegacySnapshot:
        'message.database.upgrade_stage.prepare_legacy_snapshot',
    prepareLegacyConfiguration:
        'message.database.upgrade_stage.prepare_legacy_configuration',
    finalizeLegacyMigration:
        'message.database.upgrade_stage.finalize_legacy_migration',
    initializeSchema: 'message.database.upgrade_stage.initialize_schema',
    createWorkCopy: 'message.database.upgrade_stage.create_work_copy',
    legacySchemaMigration:
        'message.database.upgrade_stage.legacy_schema_migration',
    legacyPerformanceIndexes:
        'message.database.upgrade_stage.legacy_performance_indexes',
    globalPerformanceIndexes:
        'message.database.upgrade_stage.global_performance_indexes',
    notificationPerformanceIndexes:
        'message.database.upgrade_stage.notification_performance_indexes',
    optimize: 'message.database.upgrade_stage.optimize',
    writeVersion: 'message.database.upgrade_stage.write_version',
    commit: 'message.database.upgrade_stage.commit'
};

function getDatabaseUpgradeTitleKey(phase: string): string {
    switch (phase) {
        case 'confirm-legacy-migration':
            return 'message.database.migration_found_title';
        case 'running':
            return 'message.database.upgrade_in_progress_title';
        case 'restarting':
            return 'message.database.migration_restarting_title';
        case 'error':
            return 'message.database.upgrade_failed_title';
        default:
            return 'message.database.upgrade_in_progress_title';
    }
}

function DatabaseUpgradeProgressView({
    stage,
    completed,
    total
}: {
    stage: DatabaseUpgradeStage | '';
    completed: number;
    total: number;
}) {
    const { t } = useTranslation();
    const label = stage
        ? t(DATABASE_UPGRADE_STAGE_KEYS[stage])
        : t('message.database.upgrade_in_progress_initializing');
    if (total <= 0) {
        return (
            <div
                className="text-muted-foreground flex items-center gap-3 text-sm"
                aria-live="polite"
            >
                <LoaderCircleIcon className="size-5 animate-spin motion-reduce:animate-none" />
                <span>{label}</span>
            </div>
        );
    }

    const percent = Math.min(
        100,
        Math.max(0, Math.round((completed / total) * 100))
    );
    return (
        <div className="space-y-2.5" aria-live="polite">
            <div className="flex items-center justify-between gap-4 text-sm">
                <span>{label}</span>
                <span className="text-muted-foreground tabular-nums">
                    {percent}%
                </span>
            </div>
            <Progress value={percent} aria-label={label} />
        </div>
    );
}

export function DatabaseUpgradeDialog({ open }: { open: boolean }) {
    const { t } = useTranslation();

    const databaseUpgrade = useRuntimeStore((state) => state.databaseUpgrade);
    const setDatabaseUpgradeState = useRuntimeStore(
        (state) => state.setDatabaseUpgradeState
    );
    const isBusy =
        databaseUpgrade.phase === 'running' ||
        databaseUpgrade.phase === 'restarting';
    const isBlockingFailure =
        databaseUpgrade.phase === 'error' &&
        (databaseUpgrade.retryable || databaseUpgrade.freshStartAvailable);
    const showFailureRecord =
        databaseUpgrade.phase === 'error' ||
        Boolean(databaseUpgrade.failureLogPath);

    return (
        <Dialog
            open={open}
            onOpenChange={(nextOpen) => {
                if (!nextOpen && (isBusy || isBlockingFailure)) {
                    return;
                }
                setDatabaseUpgradeState({ open: nextOpen });
            }}
        >
            <DialogContent showCloseButton={!isBusy && !isBlockingFailure}>
                <DialogHeader>
                    <DialogTitle>
                        {t(getDatabaseUpgradeTitleKey(databaseUpgrade.phase))}
                    </DialogTitle>
                    <DialogDescription>
                        {databaseUpgrade.detail ||
                            t(
                                'message.database.upgrade_in_progress_initializing'
                            )}
                    </DialogDescription>
                </DialogHeader>
                {isBusy ? (
                    <DatabaseUpgradeProgressView
                        stage={databaseUpgrade.stage}
                        completed={databaseUpgrade.progressCompleted}
                        total={databaseUpgrade.progressTotal}
                    />
                ) : null}
                {databaseUpgrade.phase !== 'confirm-legacy-migration' &&
                databaseUpgrade.phase !== 'error' &&
                (databaseUpgrade.fromVersion || databaseUpgrade.toVersion) ? (
                    <div className="bg-muted/30 text-muted-foreground rounded-md border p-3 text-sm">
                        {t('message.database.upgrade_in_progress_description', {
                            from: databaseUpgrade.fromVersion || 0,
                            to: databaseUpgrade.toVersion || 0
                        })}
                    </div>
                ) : null}
                {showFailureRecord ? (
                    <div className="space-y-3 rounded-md border p-3 text-sm">
                        <div className="space-y-1">
                            <div className="font-medium">
                                {t('message.database.failure_record')}
                            </div>
                            <div className="text-muted-foreground">
                                {t('message.database.failure_record_hint')}
                            </div>
                            <code className="bg-muted block overflow-x-auto rounded px-2 py-1.5 text-xs select-all">
                                {databaseUpgrade.failureLogPath ||
                                    'error-log.txt'}
                            </code>
                        </div>
                        <div className="flex flex-wrap gap-2">
                            <Button
                                type="button"
                                size="sm"
                                variant="outline"
                                onClick={() => {
                                    void openDatabaseUpgradeFailureLogFolder();
                                }}
                            >
                                {t('message.database.open_failure_log_folder')}
                            </Button>
                            <Button
                                type="button"
                                size="sm"
                                variant="outline"
                                onClick={() => {
                                    void createDatabaseUpgradeGitHubIssue();
                                }}
                            >
                                {t('message.database.create_github_issue')}
                            </Button>
                        </div>
                        {databaseUpgrade.failedWorkDbPath ? (
                            <div className="space-y-1 border-t pt-3">
                                <div className="font-medium">
                                    {t(
                                        'message.database.preserved_work_database'
                                    )}
                                </div>
                                <code className="bg-muted block overflow-x-auto rounded px-2 py-1.5 text-xs select-all">
                                    {databaseUpgrade.failedWorkDbPath}
                                </code>
                                <div className="text-muted-foreground text-xs">
                                    {t(
                                        'message.database.database_upload_warning'
                                    )}
                                </div>
                            </div>
                        ) : null}
                    </div>
                ) : null}
                <DialogFooter>
                    {databaseUpgrade.phase === 'confirm-legacy-migration' ? (
                        <>
                            <Button
                                type="button"
                                variant="outline"
                                onClick={() => {
                                    skipLegacyDatabaseMigration();
                                }}
                            >
                                {t('message.database.migration_skip')}
                            </Button>
                            <Button
                                type="button"
                                onClick={() => {
                                    confirmLegacyDatabaseMigration();
                                }}
                            >
                                {t('dialog.system.action.migrate_and_restart')}
                            </Button>
                        </>
                    ) : databaseUpgrade.phase === 'error' ? (
                        <>
                            {databaseUpgrade.freshStartAvailable ? (
                                <Button
                                    type="button"
                                    variant="destructive"
                                    onClick={() => {
                                        void startFreshDatabaseAfterUpgradeFailure();
                                    }}
                                >
                                    {t('message.database.use_new_database')}
                                </Button>
                            ) : null}
                            {databaseUpgrade.retryable ? (
                                <Button
                                    type="button"
                                    onClick={() => {
                                        void retryDatabaseUpgrade();
                                    }}
                                >
                                    {t('common.action.retry')}
                                </Button>
                            ) : null}
                        </>
                    ) : (
                        <Button
                            type="button"
                            variant="outline"
                            disabled={isBusy}
                            onClick={() =>
                                setDatabaseUpgradeState({ open: false })
                            }
                        >
                            {t('common.actions.close')}
                        </Button>
                    )}
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
