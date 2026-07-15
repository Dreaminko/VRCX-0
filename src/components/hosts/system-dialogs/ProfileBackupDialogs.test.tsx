// @vitest-environment jsdom

import { cleanup, render, waitFor } from '@testing-library/react';
import { StrictMode, type ComponentProps, type PropsWithChildren } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    toastError: vi.fn(),
    toastSuccess: vi.fn(),
    translate: (key: string) => key
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: mocks.translate })
}));

vi.mock('sonner', () => ({
    toast: {
        error: mocks.toastError,
        success: mocks.toastSuccess
    }
}));

vi.mock('@/services/profileBackupService', () => ({
    requestProfileRestore: vi.fn()
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
    AlertDialogMedia: ({ children }: PropsWithChildren) => (
        <div>{children}</div>
    ),
    AlertDialogTitle: ({ children }: PropsWithChildren) => <h1>{children}</h1>
}));

vi.mock('@/ui/shadcn/button', () => ({
    Button: ({ children, ...props }: ComponentProps<'button'>) => (
        <button {...props}>{children}</button>
    )
}));

import { useProfileBackupStore } from '@/state/profileBackupStore';

import { ProfileBackupDialogs } from './ProfileBackupDialogs';

describe('ProfileBackupDialogs', () => {
    beforeEach(() => {
        sessionStorage.clear();
        mocks.toastError.mockReset();
        mocks.toastSuccess.mockReset();
        useProfileBackupStore.getState().resetProfileBackupState();
    });

    afterEach(cleanup);

    it('claims an outcome once during a StrictMode mount', async () => {
        useProfileBackupStore.getState().applyStatus({
            revision: 7,
            state: 'idle',
            kind: null,
            phase: null,
            percent: null,
            error: null,
            lastOutcome: {
                revision: 7,
                kind: 'manual',
                succeeded: true,
                fileName: 'VRCX-0-manual.vrcx0backup',
                errorCode: null
            }
        });

        render(
            <StrictMode>
                <ProfileBackupDialogs />
            </StrictMode>
        );

        await waitFor(() => {
            expect(mocks.toastSuccess).toHaveBeenCalledTimes(1);
        });
        expect(mocks.toastError).not.toHaveBeenCalled();
    });
});
