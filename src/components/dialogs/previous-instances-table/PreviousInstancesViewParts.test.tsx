// @vitest-environment jsdom

import {
    cleanup,
    fireEvent,
    render,
    screen,
    waitFor
} from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

type QueryOptions = {
    enabled?: boolean;
    queryFn: () => Promise<unknown>;
};

const mocks = vi.hoisted(() => ({
    getUserProfile: vi.fn(() => Promise.resolve({})),
    knownUser: null as Record<string, unknown> | null,
    openUserDialog: vi.fn(),
    queryData: null as Record<string, unknown> | null
}));

vi.mock('@tanstack/react-query', async (importOriginal) => {
    const actual =
        await importOriginal<typeof import('@tanstack/react-query')>();
    const { useEffect } = await import('react');
    return {
        ...actual,
        useQuery: (options: QueryOptions) => {
            const { enabled, queryFn } = options;
            useEffect(() => {
                if (enabled) {
                    void queryFn();
                }
            }, [enabled, queryFn]);
            return { data: mocks.queryData };
        }
    };
});

vi.mock('@/components/layout/PageScaffold', () => ({}));
vi.mock('@/services/gameLogUserDialogService', () => ({
    openGameLogUser: vi.fn()
}));
vi.mock('@/lib/useKnownUser', () => ({
    useKnownUserFact: () => mocks.knownUser,
    useKnownUserFacts: () => ({})
}));
vi.mock('@/repositories/gameLogRepository', () => ({ default: {} }));
vi.mock('@/repositories/userProfileRepository', () => ({
    default: { getUserProfile: mocks.getUserProfile }
}));
vi.mock('@/services/dialogService', () => ({
    openUserDialog: mocks.openUserDialog,
    openWorldDialog: vi.fn()
}));
vi.mock('@/ui/shadcn/button', () => ({
    Button: ({
        children,
        onClick
    }: {
        children: ReactNode;
        onClick?: () => void;
    }) => (
        <button type="button" onClick={onClick}>
            {children}
        </button>
    )
}));
vi.mock('./PreviousInstanceInfoChart', () => ({
    PreviousInstanceInfoChart: () => null
}));

import { InstanceOwnerCell } from './PreviousInstancesViewParts';

describe('InstanceOwnerCell', () => {
    afterEach(cleanup);

    beforeEach(() => {
        vi.clearAllMocks();
        mocks.knownUser = null;
        mocks.queryData = null;
    });

    it('resolves an id-only creator through the user profile API', async () => {
        mocks.queryData = {
            id: 'usr_owner',
            displayName: 'Resolved owner'
        };

        render(
            <InstanceOwnerCell
                userId="usr_owner"
                endpoint="https://api.vrchat.cloud"
            />
        );

        await waitFor(() => {
            expect(mocks.getUserProfile).toHaveBeenCalledWith({
                userId: 'usr_owner'
            });
        });
        expect(screen.getByText('Resolved owner')).toBeTruthy();
        expect(screen.queryByText('usr_owner')).toBeNull();

        fireEvent.click(screen.getByRole('button'));
        expect(mocks.openUserDialog).toHaveBeenCalledWith({
            userId: 'usr_owner',
            title: 'Resolved owner',
            seedData: mocks.queryData
        });
    });

    it('does not refetch a creator with a known display name', () => {
        mocks.knownUser = {
            id: 'usr_owner',
            displayName: 'Known owner'
        };

        render(<InstanceOwnerCell userId="usr_owner" />);

        expect(screen.getByText('Known owner')).toBeTruthy();
        expect(mocks.getUserProfile).not.toHaveBeenCalled();
    });
});
