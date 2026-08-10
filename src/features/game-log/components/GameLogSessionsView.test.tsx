// @vitest-environment jsdom

import { cleanup, render, screen, within } from '@testing-library/react';
import type { PropsWithChildren, ReactElement } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string, values?: { count?: number }) =>
            key === 'view.game_log.sessions.friends_count'
                ? `${values?.count ?? 0} friends`
                : key
    })
}));

vi.mock('@/components/Location', () => ({
    Location: ({ hint }: { hint?: string }) => <span>{hint}</span>
}));

vi.mock('@/components/user-hover-card/UserHoverCard', () => ({
    UserHoverCard: ({ children }: PropsWithChildren) => children
}));

vi.mock('@/lib/useKnownUser', () => ({
    useKnownUserFacts: (userIds: string[]) =>
        Object.fromEntries(
            userIds.map((userId) => [
                userId,
                {
                    id: userId,
                    currentAvatarThumbnailImageUrl: `https://example.test/${userId}.png`
                }
            ])
        )
}));

vi.mock('@/services/entityMediaService', () => ({
    userImage: (user: { currentAvatarThumbnailImageUrl?: string } | null) =>
        user?.currentAvatarThumbnailImageUrl || ''
}));

vi.mock('@/ui/shadcn/avatar', () => ({
    Avatar: ({ children }: PropsWithChildren) => <span>{children}</span>,
    AvatarImage: ({ src }: { src?: string }) => <img src={src} alt="" />,
    AvatarFallback: ({ children }: PropsWithChildren) => <span>{children}</span>
}));

vi.mock('@/repositories/gameLogRepository', () => ({
    default: {
        getPlayerDetailFromInstance: vi.fn().mockResolvedValue([])
    }
}));

vi.mock('../gameLogUserLookup', () => ({
    openGameLogUser: vi.fn()
}));

vi.mock('@/ui/shadcn/collapsible', () => ({
    Collapsible: ({ children }: PropsWithChildren) => <div>{children}</div>,
    CollapsibleContent: ({ children }: PropsWithChildren) => (
        <div>{children}</div>
    ),
    CollapsibleTrigger: ({ render }: { render: ReactElement }) => render
}));

vi.mock('@/ui/shadcn/hover-card', () => ({
    HoverCard: ({ children }: PropsWithChildren) => <div>{children}</div>,
    HoverCardTrigger: ({ render }: { render: ReactElement }) => render,
    HoverCardContent: ({
        children,
        side
    }: PropsWithChildren<{ side?: string }>) => (
        <div data-testid="friends-hover-card" data-side={side}>
            {children}
        </div>
    )
}));

vi.mock('./GameLogSessionEventRow', () => ({
    SessionEventGroups: () => null
}));

import { GameLogSessionsView } from './GameLogSessionsView';

describe('GameLogSessionsView friend overflow', () => {
    afterEach(cleanup);

    it('shows every session friend with an avatar below the +n trigger', () => {
        const friends = [
            ['usr_alice', 'Alice'],
            ['usr_bob', 'Bob'],
            ['usr_carla', 'Carla'],
            ['usr_dan', 'Dan']
        ];

        render(
            <GameLogSessionsView
                isGameRunning={false}
                sessions={[
                    {
                        id: 1,
                        created_at: '2026-08-10T00:00:00.000Z',
                        duration: 60_000,
                        location: 'wrld_test:1',
                        worldName: 'Test World',
                        events: friends.map(([userId, displayName], index) => ({
                            type: 'OnPlayerJoined',
                            created_at: `2026-08-10T00:0${index}:00.000Z`,
                            userId,
                            displayName,
                            isFriend: true
                        }))
                    }
                ]}
            />
        );

        expect(
            screen.getByRole('button', { name: '4 friends' }).textContent
        ).toBe('+1');
        const hoverCard = screen.getByTestId('friends-hover-card');
        expect(hoverCard.dataset.side).toBe('bottom');

        for (const [userId, displayName] of friends) {
            const row = within(hoverCard).getByText(displayName).closest('li');
            expect(row).not.toBeNull();
            expect(row?.querySelector('img')?.getAttribute('src')).toBe(
                `https://example.test/${userId}.png`
            );
        }
    });
});
