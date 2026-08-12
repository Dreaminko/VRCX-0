// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router';
import { afterEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    queryFeedLatest: vi.fn()
}));

vi.mock('@/repositories/feedRepository', async (importOriginal) => ({
    ...(await importOriginal<typeof import('@/repositories/feedRepository')>()),
    default: {
        queryFeedLatest: mocks.queryFeedLatest
    }
}));

import { DashboardFeedWidgetView } from './DashboardFeedWidget';

describe('DashboardFeedWidgetView', () => {
    afterEach(() => {
        cleanup();
        vi.clearAllMocks();
    });

    it('renders from explicit props without reading dashboard stores', () => {
        const html = renderToStaticMarkup(
            <MemoryRouter>
                <DashboardFeedWidgetView
                    config={{}}
                    configUpdater={null}
                    currentUserId=""
                    addGameLogEventCount={0}
                    liveFeedEntries={[]}
                    liveFeedVersion={0}
                    remoteFavoriteFriendIds={[]}
                    localFriendFavorites={{}}
                    friendsById={{}}
                    feedPersistenceDisabled={false}
                />
            </MemoryRouter>
        );

        expect(html).toContain('Feed unavailable');
    });

    it('loads the Rust cache and overlays session deltas when persistence is disabled', async () => {
        mocks.queryFeedLatest.mockResolvedValue({ rows: [], maxSequence: 0 });

        render(
            <MemoryRouter>
                <DashboardFeedWidgetView
                    config={{}}
                    configUpdater={null}
                    currentUserId="usr_self"
                    addGameLogEventCount={0}
                    liveFeedEntries={[
                        {
                            sequence: 1,
                            ownerUserId: 'usr_self',
                            entry: {
                                id: 'live-only',
                                type: 'Online',
                                userId: 'usr_friend',
                                displayName: 'Friend'
                            }
                        }
                    ]}
                    liveFeedVersion={1}
                    remoteFavoriteFriendIds={[]}
                    localFriendFavorites={{}}
                    friendsById={{}}
                    feedPersistenceDisabled
                />
            </MemoryRouter>
        );

        await waitFor(() => expect(mocks.queryFeedLatest).toHaveBeenCalled());

        expect(mocks.queryFeedLatest).toHaveBeenCalledWith({
            userId: 'usr_self',
            filters: [],
            maxRows: 100
        });
        expect(screen.getByText('Friend')).toBeTruthy();
        expect(
            screen.getByRole('img', {
                name: 'Feed history is not being saved'
            })
        ).toBeTruthy();
    });

    it('groups compact feed rows by day instead of repeating the date per row', async () => {
        mocks.queryFeedLatest.mockResolvedValue({
            rows: [
                {
                    id: 'feed-1',
                    type: 'Online',
                    created_at: '2026-08-12T11:37:00.000Z',
                    userId: 'usr_one',
                    displayName: 'One'
                },
                {
                    id: 'feed-2',
                    type: 'Offline',
                    created_at: '2026-08-12T10:30:00.000Z',
                    userId: 'usr_two',
                    displayName: 'Two'
                },
                {
                    id: 'feed-3',
                    type: 'Online',
                    created_at: '2026-08-11T09:00:00.000Z',
                    userId: 'usr_three',
                    displayName: 'Three'
                }
            ],
            maxSequence: 3
        });

        const view = render(
            <MemoryRouter>
                <DashboardFeedWidgetView
                    config={{}}
                    configUpdater={null}
                    currentUserId="usr_self"
                    addGameLogEventCount={0}
                    liveFeedEntries={[]}
                    liveFeedVersion={0}
                    remoteFavoriteFriendIds={['usr_one']}
                    localFriendFavorites={{}}
                    friendsById={{}}
                    feedPersistenceDisabled={false}
                />
            </MemoryRouter>
        );

        await waitFor(() => expect(screen.getByText('Three')).toBeTruthy());

        expect(
            view.container.querySelectorAll('[data-dashboard-widget-day]')
        ).toHaveLength(2);
        expect(
            view.container.querySelector('[aria-label="Favorite"]')
        ).toBeTruthy();
        expect(screen.queryByText('Favorite')).toBeNull();
        expect(screen.queryByText('All feed types')).toBeNull();
    });
});
