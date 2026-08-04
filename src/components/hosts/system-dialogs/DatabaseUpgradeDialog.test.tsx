// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    confirmLegacyDatabaseMigration: vi.fn(),
    createDatabaseUpgradeGitHubIssue: vi.fn(),
    openDatabaseUpgradeFailureLogFolder: vi.fn(),
    retryDatabaseUpgrade: vi.fn(),
    startFreshDatabaseAfterUpgradeFailure: vi.fn(),
    skipLegacyDatabaseMigration: vi.fn()
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) => key
    })
}));

vi.mock('@/services/databaseUpgradeService', () => mocks);

vi.mock('@/ui/shadcn/button', () => ({
    Button: ({ children, ...props }: React.ComponentProps<'button'>) => (
        <button {...props}>{children}</button>
    )
}));

vi.mock('@/ui/shadcn/dialog', () => ({
    Dialog: ({ children }: React.PropsWithChildren) => <div>{children}</div>,
    DialogContent: ({ children }: React.PropsWithChildren) => (
        <section>{children}</section>
    ),
    DialogDescription: ({ children }: React.PropsWithChildren) => (
        <p>{children}</p>
    ),
    DialogFooter: ({ children }: React.PropsWithChildren) => (
        <footer>{children}</footer>
    ),
    DialogHeader: ({ children }: React.PropsWithChildren) => (
        <header>{children}</header>
    ),
    DialogTitle: ({ children }: React.PropsWithChildren) => <h1>{children}</h1>
}));

vi.mock('@/ui/shadcn/progress', () => ({
    Progress: ({
        value,
        ...props
    }: {
        value: number;
        'aria-label'?: string;
    }) => <div role="progressbar" aria-valuenow={value} {...props} />
}));

import { useRuntimeStore } from '@/state/runtimeStore';

import { DatabaseUpgradeDialog } from './DatabaseUpgradeDialog';

describe('DatabaseUpgradeDialog', () => {
    afterEach(() => {
        cleanup();
    });

    beforeEach(() => {
        vi.clearAllMocks();
        useRuntimeStore.getState().resetRuntimeState();
    });

    it('renders page-copy progress as a determinate progress bar', () => {
        useRuntimeStore.getState().setDatabaseUpgradeState({
            open: true,
            phase: 'running',
            stage: 'createWorkCopy',
            progressCompleted: 25,
            progressTotal: 100
        });

        render(<DatabaseUpgradeDialog open />);

        expect(
            screen.getByRole('progressbar').getAttribute('aria-valuenow')
        ).toBe('25');
        expect(
            screen.getByText('message.database.upgrade_stage.create_work_copy')
        ).not.toBeNull();
    });

    it('renders an animated status for an indeterminate index stage', () => {
        useRuntimeStore.getState().setDatabaseUpgradeState({
            open: true,
            phase: 'running',
            stage: 'notificationPerformanceIndexes',
            progressCompleted: 0,
            progressTotal: 0
        });

        const { container } = render(<DatabaseUpgradeDialog open />);

        expect(screen.queryByRole('progressbar')).toBeNull();
        expect(
            screen.getByText(
                'message.database.upgrade_stage.notification_performance_indexes'
            )
        ).not.toBeNull();
        expect(container.querySelector('.animate-spin')).not.toBeNull();
    });

    it('shows the failure record without hiding migration retry and skip actions', () => {
        useRuntimeStore.getState().setDatabaseUpgradeState({
            open: true,
            phase: 'confirm-legacy-migration',
            failureLogPath: 'C:/VRCX-0/error-log.txt'
        });

        render(<DatabaseUpgradeDialog open />);

        expect(screen.getByText('C:/VRCX-0/error-log.txt')).not.toBeNull();
        expect(
            screen.getByText('message.database.migration_skip')
        ).not.toBeNull();
        expect(
            screen.getByText('dialog.system.action.migrate_and_restart')
        ).not.toBeNull();
    });

    it('opens the log folder and GitHub new-issue actions from a failure', () => {
        useRuntimeStore.getState().setDatabaseUpgradeState({
            open: true,
            phase: 'error',
            failureLogPath: 'C:/VRCX-0/error-log.txt'
        });

        render(<DatabaseUpgradeDialog open />);
        fireEvent.click(
            screen.getByText('message.database.open_failure_log_folder')
        );
        fireEvent.click(
            screen.getByText('message.database.create_github_issue')
        );

        expect(mocks.openDatabaseUpgradeFailureLogFolder).toHaveBeenCalledTimes(
            1
        );
        expect(mocks.createDatabaseUpgradeGitHubIssue).toHaveBeenCalledTimes(1);
    });
});
