// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { StrictMode, type ComponentProps, type PropsWithChildren } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    takeLastResult: vi.fn(),
    toastSuccess: vi.fn()
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) =>
            ({
                'common.actions.close': 'Close',
                'profile_backup.restore_failed_rolled_back':
                    'VRCX-0 returned to the data from before the restore.',
                'profile_backup.restore_failed_title':
                    'The backup could not be restored',
                'profile_backup.restore_failure.db_open_failed':
                    'The restored database could not be opened.',
                'profile_backup.restore_succeeded': 'Data restored from backup'
            })[key] ?? key
    })
}));

vi.mock('sonner', () => ({
    toast: { success: mocks.toastSuccess }
}));

vi.mock('@/services/profileBackupService', () => ({
    takeLastProfileRestoreResult: mocks.takeLastResult
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: (
        selector: (state: {
            shell: { backendRuntimeSnapshotHydrated: boolean };
        }) => unknown
    ) => selector({ shell: { backendRuntimeSnapshotHydrated: true } })
}));

vi.mock('@/ui/shadcn/alert-dialog', () => ({
    AlertDialog: ({ children, open }: PropsWithChildren<{ open: boolean }>) =>
        open ? <div>{children}</div> : null,
    AlertDialogContent: ({ children }: PropsWithChildren) => (
        <section>{children}</section>
    ),
    AlertDialogDescription: ({ children }: PropsWithChildren) => (
        <p>{children}</p>
    ),
    AlertDialogFooter: ({ children }: PropsWithChildren) => (
        <footer>{children}</footer>
    ),
    AlertDialogHeader: ({ children }: PropsWithChildren) => (
        <header>{children}</header>
    ),
    AlertDialogTitle: ({ children }: PropsWithChildren) => <h1>{children}</h1>
}));

vi.mock('@/ui/shadcn/button', () => ({
    Button: ({ children, ...props }: ComponentProps<'button'>) => (
        <button {...props}>{children}</button>
    )
}));

import { useProfileBackupStore } from '@/state/profileBackupStore';

import { ProfileRestoreResultHost } from './ProfileRestoreResultHost';

describe('ProfileRestoreResultHost', () => {
    beforeEach(() => {
        mocks.takeLastResult.mockReset();
        mocks.toastSuccess.mockReset();
        useProfileBackupStore.getState().resetProfileBackupState();
    });

    afterEach(cleanup);

    it('takes the startup result only once across rerenders', async () => {
        mocks.takeLastResult.mockResolvedValue(null);
        const view = render(<ProfileRestoreResultHost />);

        await waitFor(() => {
            expect(mocks.takeLastResult).toHaveBeenCalledTimes(1);
        });
        view.rerender(<ProfileRestoreResultHost />);

        await waitFor(() => {
            expect(mocks.takeLastResult).toHaveBeenCalledTimes(1);
        });
    });

    it('takes the startup result only once during a StrictMode mount', async () => {
        mocks.takeLastResult.mockResolvedValue({
            status: 'succeeded',
            dataDisposition: 'replaced',
            sourceFileName: 'backup.vrcx0backup',
            failure: null
        });

        render(
            <StrictMode>
                <ProfileRestoreResultHost />
            </StrictMode>
        );

        await waitFor(() => {
            expect(mocks.toastSuccess).toHaveBeenCalledTimes(1);
        });
        expect(mocks.takeLastResult).toHaveBeenCalledTimes(1);
    });

    it('shows a dedicated typed failure dialog after rollback', async () => {
        mocks.takeLastResult.mockResolvedValue({
            status: 'failed',
            dataDisposition: 'rolledBack',
            sourceFileName: 'backup.vrcx0backup',
            failure: {
                code: 'databaseOpenFailed',
                path: null
            }
        });
        render(<ProfileRestoreResultHost />);

        expect(
            await screen.findByRole('heading', {
                name: 'The backup could not be restored'
            })
        ).toBeTruthy();
        expect(
            screen.getByText(
                'VRCX-0 returned to the data from before the restore.'
            )
        ).toBeTruthy();
        expect(
            screen.getByText('The restored database could not be opened.')
        ).toBeTruthy();
        expect(screen.getByText('backup.vrcx0backup')).toBeTruthy();
        expect(mocks.toastSuccess).not.toHaveBeenCalled();
    });

    it('ignores a failed startup result read without breaking the host', async () => {
        mocks.takeLastResult.mockRejectedValue(new Error('unavailable'));

        render(<ProfileRestoreResultHost />);

        await waitFor(() => {
            expect(mocks.takeLastResult).toHaveBeenCalledTimes(1);
        });
        expect(mocks.toastSuccess).not.toHaveBeenCalled();
        expect(
            screen.queryByRole('heading', {
                name: 'The backup could not be restored'
            })
        ).toBeNull();
    });
});
