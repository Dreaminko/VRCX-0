import type {
    DataDirMigrationErrorCode,
    DataDirMigrationPhase,
    DataDirMigrationWarning
} from './dataDirMigrationService';

export function dataDirMigrationErrorKey(
    code: DataDirMigrationErrorCode
): string {
    return `data_dir_migration.error.${code}`;
}

export function dataDirMigrationPhaseKey(
    phase: DataDirMigrationPhase | null | undefined
): string {
    return `data_dir_migration.phase.${phase ?? 'preparing'}`;
}

export function dataDirMigrationWarningKey(
    warning: DataDirMigrationWarning
): string {
    return `data_dir_migration.warning.${warning}`;
}
