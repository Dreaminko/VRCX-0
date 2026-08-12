// @vitest-environment jsdom

import { cleanup, render, screen } from '@testing-library/react';
import type { ComponentProps, PropsWithChildren, ReactElement } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) => key
    })
}));

vi.mock('@/components/affinity/AffinityBadge', () => ({
    AffinityBadge: () => null
}));

vi.mock('@/lib/dateTime', () => ({
    formatDateFilter: (value: unknown) => String(value),
    timeToText: (value: unknown) => `duration:${String(value)}`
}));

vi.mock('@/services/clipboardService', () => ({
    copyTextToClipboard: vi.fn()
}));

vi.mock('@/services/entityMediaService', () => ({
    openExternalLink: vi.fn()
}));

vi.mock('@/services/gameLogUserDialogService', () => ({
    openGameLogUser: vi.fn()
}));

vi.mock('@/ui/shadcn/button', () => ({
    Button: ({
        children,
        size: _size,
        variant: _variant,
        ...props
    }: ComponentProps<'button'> & { size?: string; variant?: string }) => (
        <button {...props}>{children}</button>
    )
}));

vi.mock('@/ui/shadcn/collapsible', () => ({
    Collapsible: ({ children }: PropsWithChildren) => <div>{children}</div>,
    CollapsibleContent: ({ children }: PropsWithChildren) => (
        <div>{children}</div>
    ),
    CollapsibleTrigger: ({ render }: { render: ReactElement }) => render
}));

import { SessionEventGroups } from './GameLogSessionEventRow';

describe('SessionEventGroups player durations', () => {
    afterEach(cleanup);

    it('shows cumulative room time only on leave events', () => {
        render(
            <SessionEventGroups
                durationByKey={new Map([['id:usr_alice', 60_000]])}
                events={[
                    {
                        type: 'OnPlayerJoined',
                        created_at: '2026-08-12T10:00:00.000Z',
                        displayName: 'Alice',
                        userId: 'usr_alice'
                    },
                    {
                        type: 'OnPlayerLeft',
                        created_at: '2026-08-12T10:01:00.000Z',
                        displayName: 'Alice',
                        userId: 'usr_alice'
                    },
                    {
                        type: 'JoinGroup',
                        created_at: '2026-08-12T10:02:00.000Z',
                        members: [
                            {
                                displayName: 'Alice',
                                userId: 'usr_alice'
                            }
                        ]
                    },
                    {
                        type: 'LeftGroup',
                        created_at: '2026-08-12T10:03:00.000Z',
                        members: [
                            {
                                displayName: 'Alice',
                                userId: 'usr_alice'
                            }
                        ]
                    }
                ]}
            />
        );

        expect(screen.getAllByText('duration:60000')).toHaveLength(2);

        const joinedRow = screen
            .getByText('2026-08-12T10:00:00.000Z')
            .closest('div');
        const leftRow = screen
            .getByText('2026-08-12T10:01:00.000Z')
            .closest('div');
        const joinedGroupMember = screen
            .getAllByText('2026-08-12T10:02:00.000Z')[1]
            .closest('div');
        const leftGroupMember = screen
            .getAllByText('2026-08-12T10:03:00.000Z')[1]
            .closest('div');

        expect(joinedRow?.textContent).not.toContain('duration:60000');
        expect(leftRow?.textContent).toContain('duration:60000');
        expect(joinedGroupMember?.textContent).not.toContain('duration:60000');
        expect(leftGroupMember?.textContent).toContain('duration:60000');
    });
});
