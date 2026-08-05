// @vitest-environment jsdom

import { cleanup, render, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => {
    const favoriteState: {
        favoriteFriendIds: string[];
        localFriendFavorites: Record<string, unknown>;
    } = {
        favoriteFriendIds: [],
        localFriendFavorites: {}
    };
    return {
        favoriteState,
        queryGameLog: vi.fn(),
        runtimeState: {
            auth: { currentUserId: 'usr_self' },
            runtimeEvents: { addGameLogEvent: { count: 0 } }
        }
    };
});

vi.mock('react-i18next', async (importOriginal) => ({
    ...(await importOriginal<typeof import('react-i18next')>()),
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock('@/repositories/gameLogRepository', () => ({
    GAME_LOG_FILTER_TYPES: [],
    default: { queryGameLog: mocks.queryGameLog }
}));

vi.mock('@/state/favoriteStore', () => ({
    useFavoriteStore: <T,>(
        selector: (state: typeof mocks.favoriteState) => T
    ) => selector(mocks.favoriteState)
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: <T,>(
        selector: (state: typeof mocks.runtimeState) => T
    ) => selector(mocks.runtimeState)
}));

import { DashboardGameLogWidget } from './DashboardGameLogWidget';

describe('DashboardGameLogWidget', () => {
    beforeEach(() => {
        mocks.runtimeState.runtimeEvents.addGameLogEvent.count = 0;
        mocks.queryGameLog.mockReset();
        mocks.queryGameLog.mockResolvedValue([]);
    });

    afterEach(cleanup);

    it('requests the visible row cap after each game-log refresh event', async () => {
        const config = { filters: ['Location'] };
        const renderWidget = () => (
            <MemoryRouter>
                <DashboardGameLogWidget config={config} />
            </MemoryRouter>
        );
        const { rerender } = render(renderWidget());

        await waitFor(() => {
            expect(mocks.queryGameLog).toHaveBeenCalledTimes(1);
            expect(mocks.queryGameLog).toHaveBeenLastCalledWith({
                currentUserId: 'usr_self',
                filters: ['Location'],
                limit: 200
            });
        });

        mocks.runtimeState.runtimeEvents.addGameLogEvent.count = 1;
        rerender(renderWidget());

        await waitFor(() => {
            expect(mocks.queryGameLog).toHaveBeenCalledTimes(2);
            expect(mocks.queryGameLog).toHaveBeenLastCalledWith({
                currentUserId: 'usr_self',
                filters: ['Location'],
                limit: 200
            });
        });
    });
});
