import i18n from '@/services/i18nService';
import { subscribeSQLiteError } from '@/shared/sqliteErrorEvents';
import { useModalStore } from '@/state/modalStore';

type SQLiteErrorCategory = 'malformed' | 'disk_full' | 'locked' | 'io_error';
type SQLiteDialogDefinition = {
    method: 'alert' | 'confirm';
    descriptionKey: string;
    titleKey: string;
};

const SQLITE_ERROR_PATTERNS = [
    {
        category: 'malformed',
        matches: ['database disk image is malformed']
    },
    { category: 'disk_full', matches: ['database or disk is full'] },
    {
        category: 'locked',
        matches: ['database is locked', 'attempt to write a readonly database']
    },
    { category: 'io_error', matches: ['disk i/o error'] }
] satisfies Array<{ category: SQLiteErrorCategory; matches: string[] }>;

const SQLITE_ERROR_DIALOGS = {
    malformed: {
        method: 'confirm',
        descriptionKey:
            'repository.sqlite_repository.modal.please_repair_or_delete_your_database_file_by_fo',
        titleKey:
            'repository.sqlite_repository.modal.your_database_is_corrupted'
    },
    disk_full: {
        method: 'alert',
        descriptionKey:
            'repository.sqlite_repository.modal.disk_full_description',
        titleKey: 'repository.sqlite_repository.modal.disk_full_title'
    },
    locked: {
        method: 'alert',
        descriptionKey:
            'repository.sqlite_repository.modal.database_locked_description',
        titleKey: 'repository.sqlite_repository.modal.database_locked_title'
    },
    io_error: {
        method: 'alert',
        descriptionKey:
            'repository.sqlite_repository.modal.disk_io_error_description',
        titleKey: 'repository.sqlite_repository.modal.disk_io_error_title'
    }
} satisfies Record<SQLiteErrorCategory, SQLiteDialogDefinition>;

const SQLITE_DIALOG_COOLDOWN_MS = 60_000;

let cleanupSQLiteErrorListener: (() => void) | null = null;
const lastShownAtByCategory = new Map<SQLiteErrorCategory, number>();

function isSQLiteErrorCategory(
    category: unknown
): category is SQLiteErrorCategory {
    return SQLITE_ERROR_PATTERNS.some((entry) => entry.category === category);
}

function getSQLiteErrorCategory(error: Error): SQLiteErrorCategory | null {
    const category =
        'sqliteCategory' in error ? error.sqliteCategory : undefined;
    if (isSQLiteErrorCategory(category)) {
        return category;
    }

    const message = error.message.toLowerCase();
    return (
        SQLITE_ERROR_PATTERNS.find(({ matches }) =>
            matches.some((pattern) => message.includes(pattern))
        )?.category ?? null
    );
}

function getSQLiteDialogDefinition(
    error: unknown
): SQLiteDialogDefinition | null {
    if (!(error instanceof Error)) {
        return null;
    }
    const category = getSQLiteErrorCategory(error);
    return category ? SQLITE_ERROR_DIALOGS[category] : null;
}

export function isKnownSQLiteError(error: unknown): boolean {
    return Boolean(getSQLiteDialogDefinition(error));
}

export async function showSQLiteErrorDialog(error: unknown): Promise<boolean> {
    if (!(error instanceof Error)) {
        return false;
    }

    const category = getSQLiteErrorCategory(error);
    if (!category) {
        return false;
    }

    const lastShownAt = lastShownAtByCategory.get(category);
    if (
        lastShownAt !== undefined &&
        Date.now() - lastShownAt < SQLITE_DIALOG_COOLDOWN_MS
    ) {
        return false;
    }
    lastShownAtByCategory.set(category, Date.now());

    const dialog = SQLITE_ERROR_DIALOGS[category];
    const modalStore = useModalStore.getState();
    try {
        await modalStore[dialog.method]({
            description: i18n.t(dialog.descriptionKey),
            title: i18n.t(dialog.titleKey)
        });
        return true;
    } catch (dialogError) {
        console.warn('Failed to show SQLite error dialog:', dialogError);
        return false;
    } finally {
        lastShownAtByCategory.set(category, Date.now());
    }
}

export function bindSQLiteErrorDialogService(): () => void {
    if (cleanupSQLiteErrorListener) {
        return cleanupSQLiteErrorListener;
    }

    const unsubscribe = subscribeSQLiteError((error: unknown) => {
        void showSQLiteErrorDialog(error);
    });

    const cleanup = () => {
        if (cleanupSQLiteErrorListener === cleanup) {
            unsubscribe();
            cleanupSQLiteErrorListener = null;
        }
    };
    cleanupSQLiteErrorListener = cleanup;
    return cleanup;
}
