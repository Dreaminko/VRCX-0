import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    alert: vi.fn(),
    confirm: vi.fn(),
    listener: null as ((error: unknown) => void) | null,
    subscribeSQLiteError: vi.fn(),
    t: vi.fn((key: string) => `translated:${key}`),
    unsubscribe: vi.fn()
}));

vi.mock('@/services/i18nService', () => ({
    default: { t: mocks.t }
}));

vi.mock('@/shared/sqliteErrorEvents', () => ({
    subscribeSQLiteError: mocks.subscribeSQLiteError
}));

vi.mock('@/state/modalStore', () => ({
    useModalStore: {
        getState: () => ({ alert: mocks.alert, confirm: mocks.confirm })
    }
}));

async function loadService() {
    vi.resetModules();
    return import('./sqliteErrorDialogService');
}

describe('sqliteErrorDialogService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.listener = null;
        mocks.alert.mockResolvedValue(undefined);
        mocks.confirm.mockResolvedValue(undefined);
        mocks.subscribeSQLiteError.mockImplementation(
            (listener: (error: unknown) => void) => {
                mocks.listener = listener;
                return mocks.unsubscribe;
            }
        );
    });

    it.each([
        {
            message: 'database disk image is malformed',
            method: 'confirm',
            titleKey:
                'repository.sqlite_repository.modal.your_database_is_corrupted'
        },
        {
            message: 'SQLite failure: database or disk is full',
            method: 'alert',
            titleKey: 'repository.sqlite_repository.modal.disk_full_title'
        },
        {
            message: 'Database is locked',
            method: 'alert',
            titleKey: 'repository.sqlite_repository.modal.database_locked_title'
        },
        {
            message: 'attempt to write a readonly database',
            method: 'alert',
            titleKey: 'repository.sqlite_repository.modal.database_locked_title'
        },
        {
            message: 'Disk I/O error',
            method: 'alert',
            titleKey: 'repository.sqlite_repository.modal.disk_io_error_title'
        }
    ])(
        'recognizes $message errors from the current IPC boundary',
        async ({ message, method, titleKey }) => {
            const { isKnownSQLiteError, showSQLiteErrorDialog } =
                await loadService();
            const error = new Error(message);

            expect(isKnownSQLiteError(error)).toBe(true);
            await expect(showSQLiteErrorDialog(error)).resolves.toBe(true);
            expect(
                method === 'confirm' ? mocks.confirm : mocks.alert
            ).toHaveBeenCalledWith(
                expect.objectContaining({ title: `translated:${titleKey}` })
            );
        }
    );

    it('prefers a structured category and ignores unknown errors', async () => {
        const { isKnownSQLiteError, showSQLiteErrorDialog } =
            await loadService();
        const categorized = Object.assign(new Error('opaque backend error'), {
            sqliteCategory: 'disk_full'
        });

        expect(isKnownSQLiteError(categorized)).toBe(true);
        await expect(showSQLiteErrorDialog(categorized)).resolves.toBe(true);
        await expect(
            showSQLiteErrorDialog(new Error('permission denied'))
        ).resolves.toBe(false);
        expect(mocks.alert).toHaveBeenCalledTimes(1);
    });

    it('collapses repeated failures of the same category', async () => {
        const { showSQLiteErrorDialog } = await loadService();

        await expect(
            showSQLiteErrorDialog(new Error('database is locked'))
        ).resolves.toBe(true);
        await expect(
            showSQLiteErrorDialog(new Error('database is locked'))
        ).resolves.toBe(false);
        await expect(
            showSQLiteErrorDialog(
                new Error('attempt to write a readonly database')
            )
        ).resolves.toBe(false);
        await expect(
            showSQLiteErrorDialog(new Error('database or disk is full'))
        ).resolves.toBe(true);
        expect(mocks.alert).toHaveBeenCalledTimes(2);
    });

    it('shows a category again once its cooldown expires', async () => {
        vi.useFakeTimers();
        try {
            const { showSQLiteErrorDialog } = await loadService();

            await expect(
                showSQLiteErrorDialog(new Error('disk I/O error'))
            ).resolves.toBe(true);
            vi.advanceTimersByTime(59_000);
            await expect(
                showSQLiteErrorDialog(new Error('disk I/O error'))
            ).resolves.toBe(false);
            vi.advanceTimersByTime(2_000);
            await expect(
                showSQLiteErrorDialog(new Error('disk I/O error'))
            ).resolves.toBe(true);
            expect(mocks.alert).toHaveBeenCalledTimes(2);
        } finally {
            vi.useRealTimers();
        }
    });

    it('binds once and can rebind after either caller cleans up', async () => {
        const { bindSQLiteErrorDialogService } = await loadService();

        const firstCleanup = bindSQLiteErrorDialogService();
        const secondCleanup = bindSQLiteErrorDialogService();
        expect(secondCleanup).toBe(firstCleanup);
        expect(mocks.subscribeSQLiteError).toHaveBeenCalledTimes(1);

        secondCleanup();
        expect(mocks.unsubscribe).toHaveBeenCalledTimes(1);
        bindSQLiteErrorDialogService();
        expect(mocks.subscribeSQLiteError).toHaveBeenCalledTimes(2);
    });

    it('routes subscribed errors through the dialog service', async () => {
        const { bindSQLiteErrorDialogService } = await loadService();

        bindSQLiteErrorDialogService();
        mocks.listener?.(new Error('database or disk is full'));

        await vi.waitFor(() => expect(mocks.alert).toHaveBeenCalledTimes(1));
    });
});
