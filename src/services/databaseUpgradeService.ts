import { toast } from 'sonner';

import {
    commands,
    type DatabaseUpgradePreflight,
    type DatabaseUpgradeProgress,
    type DatabaseUpgradeRunResult,
    type DatabaseUpgradeStatus,
    type LegacyVrcxMigrationStatus
} from '@/platform/tauri/bindings';
import configRepository from '@/repositories/configRepository';
import i18n from '@/services/i18nService';
import { openExternalLink } from '@/services/shellIntegrationService';
import { links } from '@/shared/constants/link';
import { useModalStore } from '@/state/modalStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';

import { showSQLiteErrorDialog } from './sqliteErrorDialogService';

type DatabaseUpgradePatch = Parameters<
    ReturnType<typeof useRuntimeStore.getState>['setDatabaseUpgradeState']
>[0];
type FailureRecoveryOptions = {
    retryable?: boolean;
    freshStartAvailable?: boolean;
};

const DATABASE_UPGRADE_ISSUE_URL = `${links.issues}/new?template=bug_report.yml`;
const DATABASE_UPGRADE_PROGRESS_POLL_INTERVAL_MS = 100;

function setUpgradeState(patch: DatabaseUpgradePatch): void {
    useRuntimeStore.getState().setDatabaseUpgradeState(patch);
}

function applyDatabaseUpgradeProgress(
    progress: DatabaseUpgradeProgress
): void {
    setUpgradeState({
        stage: progress.stage,
        progressCompleted: progress.completedUnits ?? 0,
        progressTotal: progress.totalUnits ?? 0
    });
}

function startDatabaseUpgradeProgressPolling(): () => void {
    let requestInFlight = false;
    let stopped = false;
    async function refresh(): Promise<void> {
        if (requestInFlight || stopped) {
            return;
        }
        requestInFlight = true;
        try {
            const progress = await commands.appDatabaseUpgradeProgress();
            if (!stopped) {
                applyDatabaseUpgradeProgress(progress);
            }
        } catch (error) {
            console.warn('Database upgrade progress unavailable:', error);
        } finally {
            requestInFlight = false;
        }
    }

    void refresh();
    const timer = globalThis.setInterval(() => {
        void refresh();
    }, DATABASE_UPGRADE_PROGRESS_POLL_INTERVAL_MS);
    return () => {
        stopped = true;
        globalThis.clearInterval(timer);
    };
}

function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

function failedUpgradeDescription(
    failedUpgrade: DatabaseUpgradeStatus | null | undefined
): string {
    const workDbPath =
        failedUpgrade?.workDbPath ||
        i18n.t('service.database_upgrade_service.label.unknown_path');
    if (failedUpgrade?.reason) {
        return i18n.t(
            'service.database_upgrade_service.error.failed_upgrade_description_with_reason',
            {
                path: workDbPath,
                reason: String(failedUpgrade.reason)
            }
        );
    }
    return i18n.t(
        'service.database_upgrade_service.error.failed_upgrade_description',
        { path: workDbPath }
    );
}

async function getFailureLogPath(): Promise<string> {
    try {
        return await commands.appDatabaseUpgradeFailureLogPath();
    } catch (error) {
        console.warn('Database upgrade failure log path unavailable:', error);
        return '';
    }
}

async function blockOnFailedUpgrade(
    failedUpgrade: DatabaseUpgradeStatus | null | undefined,
    fallbackDescription?: string,
    versions?: { fromVersion: number; toVersion: number },
    options: FailureRecoveryOptions = {}
): Promise<boolean> {
    const current = useRuntimeStore.getState().databaseUpgrade;
    const retryable = options.retryable ?? true;
    const freshStartAvailable =
        options.freshStartAvailable ?? Boolean(failedUpgrade);
    const description = failedUpgrade
        ? failedUpgradeDescription(failedUpgrade)
        : fallbackDescription ||
          i18n.t('service.database_upgrade_service.error.apply_upgrade_failed');
    const logPath = (await getFailureLogPath()) || current.failureLogPath;
    setUpgradeState({
        open: true,
        phase: 'error',
        fromVersion: failedUpgrade?.fromVersion ?? versions?.fromVersion ?? 0,
        toVersion: failedUpgrade?.toVersion ?? versions?.toVersion ?? 0,
        stage: current.stage,
        progressCompleted: 0,
        progressTotal: 0,
        detail: description,
        legacyMigrationAvailable: false,
        retryable,
        freshStartAvailable,
        failureLogPath: logPath,
        failedWorkDbPath:
            failedUpgrade?.workDbPath || current.failedWorkDbPath
    });
    useSessionStore.getState().setSessionState({ databaseReady: false });
    return false;
}

function setRunningState(
    preflight?: DatabaseUpgradePreflight,
    forceOpen = false
): void {
    const fromVersion = preflight?.fromVersion ?? 0;
    const toVersion = preflight?.toVersion ?? 0;
    const shouldShowProgress =
        forceOpen ||
        preflight?.status === 'upgradeRequired' ||
        preflight?.status === 'running';
    setUpgradeState({
        open: shouldShowProgress,
        phase: 'running',
        fromVersion,
        toVersion,
        stage: 'preflight',
        progressCompleted: 0,
        progressTotal: 0,
        detail: i18n.t(
            'service.database_upgrade_service.dynamic.updating_database_from_value_to_value',
            { value: fromVersion, value2: toVersion }
        ),
        legacyMigrationAvailable: false,
        retryable: false,
        freshStartAvailable: false,
        failureLogPath: '',
        failedWorkDbPath: ''
    });
}

async function completeDatabaseUpgrade(
    result: DatabaseUpgradeRunResult
): Promise<boolean> {
    if (result.status === 'upgraded') {
        try {
            await configRepository.reload();
        } catch (error) {
            console.error(
                'Config refresh failed after database upgrade:',
                error
            );
            await showSQLiteErrorDialog(error);
            return blockOnFailedUpgrade(
                null,
                i18n.t(
                    'service.database_upgrade_service.action.refresh_config_failed_after_upgrade'
                ),
                result,
                { freshStartAvailable: false }
            );
        }
    }

    if (result.repairWarning) {
        console.warn(
            'Co-presence duration repair will be retried on the next startup:',
            result.repairWarning
        );
    }

    setUpgradeState({
        open: false,
        phase: 'completed',
        fromVersion: result.fromVersion,
        toVersion: result.toVersion,
        stage: 'commit',
        progressCompleted: 0,
        progressTotal: 0,
        detail:
            result.status === 'upgraded'
                ? i18n.t(
                      'service.database_upgrade_service.success.database_update_complete'
                  )
                : i18n.t(
                      'service.database_upgrade_service.label.database_schema_is_current'
                  ),
        legacyMigrationAvailable: false,
        retryable: false,
        freshStartAvailable: false,
        failureLogPath: '',
        failedWorkDbPath: ''
    });
    useSessionStore.getState().setSessionState({ databaseReady: true });
    return true;
}

async function handleDatabaseUpgradeResult(
    result: DatabaseUpgradeRunResult
): Promise<boolean> {
    if (result.status === 'current' || result.status === 'upgraded') {
        return completeDatabaseUpgrade(result);
    }

    if (result.status === 'failed') {
        const error = new Error(
            result.error ||
                i18n.t(
                    'service.database_upgrade_service.error.apply_upgrade_failed'
                )
        );
        console.error('Database upgrade failed:', error);
        await showSQLiteErrorDialog(error);
    }

    return blockOnFailedUpgrade(
        result.failedUpgrade,
        result.error ||
            i18n.t(
                'service.database_upgrade_service.error.apply_upgrade_failed'
        ),
        result,
        {
            retryable: result.status !== 'newerSchema',
            freshStartAvailable: true
        }
    );
}

async function runBackendDatabaseUpgrade(
    preflight?: DatabaseUpgradePreflight
): Promise<boolean> {
    setRunningState(preflight);
    const stopProgressPolling = startDatabaseUpgradeProgressPolling();
    try {
        const result = await commands.appDatabaseUpgradeRun();
        return handleDatabaseUpgradeResult(result);
    } catch (error) {
        console.error('Database upgrade command failed:', error);
        await showSQLiteErrorDialog(error);
        return blockOnFailedUpgrade(
            null,
            `${i18n.t(
                'service.database_upgrade_service.error.apply_upgrade_failed'
            )} ${errorMessage(error)}`,
            preflight
        );
    } finally {
        stopProgressPolling();
    }
}

export async function retryDatabaseUpgrade(): Promise<boolean> {
    const { fromVersion, toVersion } =
        useRuntimeStore.getState().databaseUpgrade;
    const preflight: DatabaseUpgradePreflight = {
        status: 'upgradeRequired',
        fromVersion,
        toVersion
    };
    setRunningState(preflight, true);
    const stopProgressPolling = startDatabaseUpgradeProgressPolling();
    try {
        const result = await commands.appDatabaseUpgradeRetry();
        return handleDatabaseUpgradeResult(result);
    } catch (error) {
        console.error('Database upgrade retry command failed:', error);
        await showSQLiteErrorDialog(error);
        return blockOnFailedUpgrade(
            null,
            `${i18n.t(
                'service.database_upgrade_service.error.apply_upgrade_failed'
            )} ${errorMessage(error)}`,
            preflight,
            { freshStartAvailable: true }
        );
    } finally {
        stopProgressPolling();
    }
}

export async function openDatabaseUpgradeFailureLogFolder(): Promise<void> {
    await commands.appOpenVrcxAppDataFolder();
}

export async function createDatabaseUpgradeGitHubIssue(): Promise<void> {
    await openExternalLink(DATABASE_UPGRADE_ISSUE_URL);
}

export async function startFreshDatabaseAfterUpgradeFailure(): Promise<boolean> {
    const before = useRuntimeStore.getState().databaseUpgrade;
    if (!before.freshStartAvailable) {
        return false;
    }
    const confirmation = await useModalStore.getState().confirm({
        title: i18n.t('message.database.fresh_start_confirm_title'),
        description: i18n.t(
            'message.database.fresh_start_confirm_description'
        ),
        confirmText: i18n.t('message.database.use_new_database'),
        cancelText: i18n.t('common.actions.cancel'),
        destructive: true
    });
    if (!confirmation.ok) {
        return false;
    }

    setUpgradeState({
        open: true,
        phase: 'restarting',
        detail: i18n.t('message.database.fresh_start_restarting'),
        retryable: false,
        freshStartAvailable: false
    });
    try {
        await commands.appDatabaseUpgradeStartFresh();
        return true;
    } catch (error) {
        console.error('Fresh database recovery failed:', error);
        await showSQLiteErrorDialog(error);
        setUpgradeState({
            ...before,
            open: true,
            phase: 'error',
            detail: `${i18n.t(
                'message.database.fresh_start_failed'
            )} ${errorMessage(error)}`
        });
        return false;
    }
}

async function getLegacyMigrationStatus(): Promise<LegacyVrcxMigrationStatus> {
    try {
        return commands.appGetLegacyVrcxMigrationStatus();
    } catch (error) {
        console.warn('Legacy VRCX migration status check failed:', error);
    }

    try {
        const available = Boolean(await commands.appCheckLegacyVrcxAvailable());
        return {
            detected: available,
            available
        };
    } catch (error) {
        console.warn('Legacy VRCX availability check failed:', error);
        return {
            detected: false,
            available: false
        };
    }
}

export async function initializeDatabaseUpgradeFlow(): Promise<boolean> {
    let preflight: DatabaseUpgradePreflight;
    try {
        preflight = await commands.appDatabaseUpgradePreflight();
    } catch (error) {
        console.error('Database upgrade preflight failed:', error);
        throw error;
    }

    if (preflight.status === 'running') {
        return runBackendDatabaseUpgrade(preflight);
    }
    if (preflight.status === 'finished') {
        if (!preflight.result) {
            throw new Error(
                'Finished database upgrade status is missing its result.'
            );
        }
        return handleDatabaseUpgradeResult(preflight.result);
    }

    if (preflight.status === 'blocked') {
        return blockOnFailedUpgrade(preflight.failedUpgrade);
    }
    if (preflight.status === 'newerSchema') {
        return blockOnFailedUpgrade(
            null,
            i18n.t(
                'service.database_upgrade_service.error.newer_schema_requires_newer_app',
                {
                    value: preflight.fromVersion,
                    value2: preflight.toVersion
                }
            ),
            preflight,
            {
                retryable: false,
                freshStartAvailable: true
            }
        );
    }

    const legacyMigrationStatus = await getLegacyMigrationStatus();

    if (legacyMigrationStatus.available) {
        setUpgradeState({
            open: true,
            phase: 'confirm-legacy-migration',
            fromVersion: preflight.fromVersion,
            toVersion: preflight.toVersion,
            detail: i18n.t('message.database.migration_found_description'),
            legacyMigrationAvailable: true,
            retryable: false,
            freshStartAvailable: false,
            failureLogPath: '',
            failedWorkDbPath: ''
        });
        useSessionStore.getState().setSessionState({ databaseReady: false });
        return false;
    }

    if (legacyMigrationStatus.detected && legacyMigrationStatus.reason) {
        toast.warning(legacyMigrationStatus.reason);
    }

    return runBackendDatabaseUpgrade(preflight);
}

export async function confirmLegacyDatabaseMigration(): Promise<void> {
    let failureDetail = i18n.t(
        'service.database_upgrade_service.error.legacy_migration_restart_failed'
    );
    setUpgradeState({
        open: true,
        phase: 'running',
        stage: 'prepareLegacySnapshot',
        progressCompleted: 0,
        progressTotal: 0,
        detail: i18n.t(
            'service.database_upgrade_service.action.requesting_legacy_migration'
        ),
        retryable: false,
        freshStartAvailable: false,
        failureLogPath: '',
        failedWorkDbPath: ''
    });

    const stopProgressPolling = startDatabaseUpgradeProgressPolling();
    try {
        const willRestart = await commands.appRequestLegacyMigration();
        if (willRestart) {
            setUpgradeState({
                phase: 'restarting',
                stage: 'finalizeLegacyMigration',
                progressCompleted: 0,
                progressTotal: 0
            });
            return;
        }
    } catch (error) {
        console.error('Legacy migration request failed:', error);
        const failureLogPath = await getFailureLogPath();
        failureDetail = `${failureDetail} ${errorMessage(error)}`;
        setUpgradeState({
            failureLogPath
        });
    } finally {
        stopProgressPolling();
    }

    setUpgradeState({
        open: true,
        phase: 'confirm-legacy-migration',
        stage: '',
        progressCompleted: 0,
        progressTotal: 0,
        detail: failureDetail,
        retryable: false
    });
}

export async function skipLegacyDatabaseMigration(): Promise<boolean> {
    const { fromVersion, toVersion } =
        useRuntimeStore.getState().databaseUpgrade;
    return runBackendDatabaseUpgrade({
        status: 'upgradeRequired',
        fromVersion,
        toVersion
    });
}
