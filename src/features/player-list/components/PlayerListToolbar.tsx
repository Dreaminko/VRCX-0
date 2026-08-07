import { useTranslation } from 'react-i18next';

import type { AppTable } from '@/components/data-table/appTable';
import { TableColumnVisibilityMenu } from '@/components/data-table/TableColumnVisibilityMenu';
import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarSearch,
    ToolbarSegmented,
    ToolbarViews,
    type ToolbarSegmentOption
} from '@/components/layout/ToolbarControls';

import type {
    PlayerListFilterScope,
    PlayerListScopeCounts
} from '../playerListFilters';
import type { PlayerListRow } from '../playerListTypes';

const FILTER_SCOPES = [
    'all',
    'friend',
    'favorite',
    'restricted'
] satisfies readonly PlayerListFilterScope[];

type PlayerListToolbarProps = {
    counts: PlayerListScopeCounts;
    onQueryChange: (query: string) => void;
    onResetLayout: () => void;
    onScopeChange: (scope: PlayerListFilterScope) => void;
    query: string;
    scope: PlayerListFilterScope;
    table: AppTable<PlayerListRow>;
};

export function PlayerListToolbar({
    counts,
    onQueryChange,
    onResetLayout,
    onScopeChange,
    query,
    scope,
    table
}: PlayerListToolbarProps) {
    const { t } = useTranslation();
    const scopeLabels: Record<PlayerListFilterScope, string> = {
        all: t('view.player_list.label.all'),
        friend: t('view.player_list.label.friends'),
        favorite: t('view.player_list.label.favorites'),
        restricted: t('view.player_list.label.restricted')
    };
    const scopeOptions: ToolbarSegmentOption<PlayerListFilterScope>[] =
        FILTER_SCOPES.map((filterScope) => ({
            value: filterScope,
            label: scopeLabels[filterScope],
            count: counts[filterScope]
        }));

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    <ToolbarSegmented
                        value={scope}
                        onValueChange={onScopeChange}
                        options={scopeOptions}
                    />
                </ToolbarViews>

                <ToolbarSearch
                    value={query}
                    onValueChange={onQueryChange}
                    placeholder={t('view.player_list.label.search_placeholder')}
                    ariaLabel={`${t('common.actions.search')} · ${t('nav_tooltip.player_list')}`}
                />

                <ToolbarActions>
                    <TableColumnVisibilityMenu
                        table={table}
                        onResetLayout={onResetLayout}
                    />
                </ToolbarActions>
            </PageToolbarRow>
        </PageToolbar>
    );
}
