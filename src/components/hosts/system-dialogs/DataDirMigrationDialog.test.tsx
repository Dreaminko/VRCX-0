// @vitest-environment jsdom

import {
    act,
    cleanup,
    fireEvent,
    render,
    screen
} from '@testing-library/react';
import type { ComponentProps, PropsWithChildren } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    cancelMigration: vi.fn(),
    requestMigration: vi.fn(),
    restartApplication: vi.fn(),
    setSystemHostOpen: vi.fn(),
    toastError: vi.fn()
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) => key,
        i18n: { language: 'en' }
    })
}));

vi.mock('sonner', () => ({
    toast: { error: mocks.toastError }
}));

vi.mock('@/services/dataDirMigrationService', () => ({
    cancelDataDirMigration: mocks.cancelMigration,
    requestDataDirMigration: mocks.requestMigration
}));

vi.mock('@/services/shellIntegrationService', () => ({
    restartApplication: mocks.restartApplication
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: <T,>(
        selector: (state: {
            setSystemHostOpen: typeof mocks.setSystemHostOpen;
        }) => T
    ) => selector({ setSystemHostOpen: mocks.setSystemHostOpen })
}));

vi.mock('@/ui/shadcn/alert-dialog', () => ({
    AlertDialog: ({ children, open }: PropsWithChildren<{ open: boolean }>) =>
        open ? <>{children}</> : null,
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
    Button: ({
        children,
        variant: _variant,
        ...props
    }: PropsWithChildren<ComponentProps<'button'> & { variant?: string }>) => (
        <button {...props}>{children}</button>
    )
}));

vi.mock('@/ui/shadcn/progress', () => ({
    Progress: ({ value }: { value: number }) => (
        <div aria-label="migration-progress" data-value={value} />
    )
}));

import { useDataDirMigrationStore } from '@/state/dataDirMigrationStore';

import { DataDirMigrationDialog } from './DataDirMigrationDialog';

const plan = {
    targetPath: 'D:\\VRCX-0',
    requiredBytes: 2048,
    availableBytes: 1024,
    targetState: 'empty' as const
};

describe('DataDirMigrationDialog', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        useDataDirMigrationStore.getState().closeDialog();
        useDataDirMigrationStore.setState({
            status: { revision: 0, state: 'idle' },
            lastAppliedRevision: -1
        });
    });

    afterEach(cleanup);

    it('does not render without an active migration plan', () => {
        render(<DataDirMigrationDialog />);
        expect(
            screen.queryByRole('heading', { name: 'data_dir_migration.title' })
        ).toBeNull();
    });

    it('marks storage risks as dangerous and blocks only an unsafe migrate start', () => {
        useDataDirMigrationStore.getState().openDialog(plan);
        render(<DataDirMigrationDialog />);

        expect(
            screen
                .getByText('data_dir_migration.insufficient_space')
                .classList.contains('text-destructive')
        ).toBe(true);
        expect(
            screen
                .getByText('data_dir_migration.unsupported_storage_warning')
                .classList.contains('text-destructive')
        ).toBe(true);
        expect(
            (
                screen.getByRole('button', {
                    name: 'data_dir_migration.start'
                }) as HTMLButtonElement
            ).disabled
        ).toBe(true);

        fireEvent.click(
            screen.getByRole('radio', {
                name: 'data_dir_migration.mode.freshStart'
            })
        );
        expect(
            screen.queryByText('data_dir_migration.insufficient_space')
        ).toBeNull();
        expect(
            screen
                .getByText('data_dir_migration.unsupported_storage_warning')
                .classList.contains('text-destructive')
        ).toBe(true);
        expect(
            (
                screen.getByRole('button', {
                    name: 'data_dir_migration.start'
                }) as HTMLButtonElement
            ).disabled
        ).toBe(false);
    });

    it('allows cancellation only during the copying phase', () => {
        const store = useDataDirMigrationStore.getState();
        store.openDialog({ ...plan, availableBytes: 4096 });
        store.applyStatus({
            revision: 1,
            state: 'running',
            phase: 'verifying',
            percent: 80
        });
        render(<DataDirMigrationDialog />);

        expect(
            (
                screen.getByRole('button', {
                    name: 'common.actions.cancel'
                }) as HTMLButtonElement
            ).disabled
        ).toBe(true);
        expect(
            screen
                .getByLabelText('migration-progress')
                .getAttribute('data-value')
        ).toBe('80');

        act(() => {
            useDataDirMigrationStore.getState().applyStatus({
                revision: 2,
                state: 'running',
                phase: 'copying',
                percent: 40
            });
        });
        expect(
            (
                screen.getByRole('button', {
                    name: 'common.actions.cancel'
                }) as HTMLButtonElement
            ).disabled
        ).toBe(false);
    });

    it('replaces migration controls with restart choices after completion', () => {
        const store = useDataDirMigrationStore.getState();
        store.openDialog({ ...plan, availableBytes: 4096 });
        store.applyStatus({ revision: 1, state: 'completed' });
        render(<DataDirMigrationDialog />);

        expect(
            screen.queryByRole('button', { name: 'data_dir_migration.start' })
        ).toBeNull();
        for (const name of [
            'data_dir_migration.restart_later',
            'data_dir_migration.restart_now'
        ]) {
            expect(
                (screen.getByRole('button', { name }) as HTMLButtonElement)
                    .disabled
            ).toBe(false);
        }
    });
});
