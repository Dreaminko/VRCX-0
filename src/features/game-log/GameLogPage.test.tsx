// @vitest-environment jsdom

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { GameLogPage } from './GameLogPage';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) =>
            ({
                'view.game_log.label.game_log_is_disabled':
                    'You have turned off GameLog writing',
                'view.game_log.action.enable_game_log_ingestion_in_settings_before_this_page_can_load_local_vrchat_activity':
                    'New records are not saved; history remains available.'
            })[key] ?? key
    })
}));

vi.mock('@/components/dialogs/PreviousInstancesTableDialog', () => ({
    PreviousInstancesTableDialog: () => null
}));

vi.mock('./components/GameLogToolbar', () => ({
    GameLogToolbar: ({
        refreshModel
    }: {
        refreshModel: { canRefresh: boolean };
    }) => (
        <div data-testid="toolbar" data-can-refresh={refreshModel.canRefresh} />
    )
}));

vi.mock('./components/GameLogTableShell', () => ({
    GameLogTableShell: ({ rows }: { rows: unknown[] }) => (
        <div>History rows: {rows.length}</div>
    )
}));

vi.mock('./components/GameLogSessionsView', () => ({
    GameLogSessionsView: () => <div>Sessions</div>
}));

vi.mock('./components/GameLogTableParts', () => ({
    GameLogEmptyState: ({ title }: { title: string }) => <div>{title}</div>
}));

vi.mock('./useGameLogPageController', () => ({
    useGameLogPageController: () => ({
        annotations: {
            annotatedRows: [{ id: 1 }],
            annotatedSessions: []
        },
        filters: {
            deferredSearchQuery: '',
            favoritesOnly: false,
            queryFilterTypes: [],
            refreshGameLog: vi.fn(),
            sessionDateFrom: '',
            sessionDateTo: '',
            viewMode: 'rows'
        },
        hasMoreSessions: false,
        isError: false,
        isGameRunning: false,
        isLoading: false,
        isLoadingMoreSessions: false,
        pageCount: 1,
        previousInstancesDialog: {
            open: false,
            rows: [],
            setOpen: vi.fn(),
            setRows: vi.fn(),
            title: ''
        },
        rowsState: {
            currentUserId: 'usr_test',
            detail: '',
            gameLogDisabled: true,
            isFavoritesLoaded: true,
            loadStatus: 'ready'
        },
        table: {},
        tableState: {
            loadMoreSessions: vi.fn(),
            pageSizes: [10],
            setPagination: vi.fn(),
            setSessionLimit: vi.fn()
        }
    })
}));

describe('GameLogPage', () => {
    afterEach(cleanup);

    it('keeps history and refresh available while showing the write warning', () => {
        render(<GameLogPage />);

        expect(
            screen.getByText('You have turned off GameLog writing')
        ).toBeTruthy();
        expect(
            screen.getByText(
                'New records are not saved; history remains available.'
            )
        ).toBeTruthy();
        expect(screen.getByText('History rows: 1')).toBeTruthy();
        expect(
            screen.getByTestId('toolbar').getAttribute('data-can-refresh')
        ).toBe('true');
    });
});
