import { commands } from '@/platform/tauri/bindings';
import type { LegacyVrcxMigrationStatus } from '@/platform/tauri/bindings';

type ConfirmResult = {
    ok?: boolean;
    reason?: string;
};
type ConfirmOptions = Record<string, unknown> & {
    title: string;
    description: string;
    confirmText: string;
    alternativeText?: string;
    cancelText?: string;
    dismissible?: boolean;
    destructive?: boolean;
};
type LegacyMigrationPromptOptions = {
    alert: (options: ConfirmOptions) => Promise<ConfirmResult>;
    confirm: (options: ConfirmOptions) => Promise<ConfirmResult>;
    t: (key: string, params?: Record<string, unknown>) => string;
    toast: {
        error: (message: string) => unknown;
        warning: (message: string) => unknown;
    };
};

function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

const LEGACY_MIGRATION_I18N_PREFIX =
    'view.settings.advanced.advanced.database_cleanup';
const LEGACY_PROCESS_I18N_PREFIX = 'message.database';

export async function confirmLegacyVrcxProcessState({
    alert,
    t
}: Pick<LegacyMigrationPromptOptions, 'alert' | 't'>): Promise<boolean> {
    while (await commands.appIsLegacyVrcxRunning()) {
        const result = await alert({
            title: t(`${LEGACY_PROCESS_I18N_PREFIX}.legacy_vrcx_running_title`),
            description: t(
                `${LEGACY_PROCESS_I18N_PREFIX}.legacy_vrcx_running_description`
            ),
            confirmText: t(
                `${LEGACY_PROCESS_I18N_PREFIX}.legacy_vrcx_force_migration`
            ),
            alternativeText: t(
                `${LEGACY_PROCESS_I18N_PREFIX}.legacy_vrcx_check_again`
            ),
            dismissible: false,
            destructive: true
        });
        if (result.reason !== 'alternative') {
            return result.ok === true && result.reason === 'ok';
        }
    }
    return false;
}

export async function promptLegacyVrcxForceMigration({
    alert,
    confirm,
    t,
    toast
}: LegacyMigrationPromptOptions): Promise<void> {
    let status: LegacyVrcxMigrationStatus | null = null;
    try {
        status = await commands.appGetLegacyVrcxForceMigrationStatus();
    } catch (error) {
        toast.error(
            t(`${LEGACY_MIGRATION_I18N_PREFIX}.legacy_migration_failed`, {
                error: errorMessage(error)
            })
        );
        return;
    }

    if (!status?.available) {
        toast.error(
            status?.reason ||
                t(
                    `${LEGACY_MIGRATION_I18N_PREFIX}.legacy_migration_not_available`
                )
        );
        return;
    }

    const result = await confirm({
        title: t(
            `${LEGACY_MIGRATION_I18N_PREFIX}.legacy_migration_confirm_title`
        ),
        description: t(
            `${LEGACY_MIGRATION_I18N_PREFIX}.legacy_migration_confirm_description`,
            {
                path: status.dbPath || '%APPDATA%\\VRCX'
            }
        ),
        confirmText: t(
            `${LEGACY_MIGRATION_I18N_PREFIX}.legacy_migration_confirm`
        ),
        cancelText: t('common.actions.cancel'),
        destructive: true
    });
    if (!result.ok) {
        return;
    }

    try {
        const allowRunningLegacyVrcx = await confirmLegacyVrcxProcessState({
            alert,
            t
        });
        const willRestart = await commands.appRequestLegacyVrcxForceMigration(
            allowRunningLegacyVrcx
        );
        if (!willRestart) {
            toast.warning(
                t(
                    `${LEGACY_MIGRATION_I18N_PREFIX}.legacy_migration_restart_manually`
                )
            );
        }
    } catch (error) {
        toast.error(
            t(`${LEGACY_MIGRATION_I18N_PREFIX}.legacy_migration_failed`, {
                error: errorMessage(error)
            })
        );
    }
}
