import type { Table } from '@tanstack/react-table';
import { useTranslation } from 'react-i18next';

import { TableColumnVisibilityMenu } from '@/components/data-table/TableColumnVisibilityMenu';
import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarRefreshButton,
    ToolbarSearch,
    ToolbarStatus,
    ToolbarViews
} from '@/components/layout/ToolbarControls';

import type { FriendLogRow } from '../friendLogRows';
import { FriendLogTypeFilterDropdown } from './FriendLogViewParts';

export function FriendLogPageToolbar({
    selectedTypes,
    onSelectedTypesChange,
    searchQuery,
    onSearchQueryChange,
    detail,
    currentUserId,
    loadStatus,
    onRefresh,
    table
}: {
    selectedTypes: string[];
    onSelectedTypesChange: (value: string[]) => void;
    searchQuery: string;
    onSearchQueryChange: (value: string) => void;
    detail: string;
    currentUserId: string;
    loadStatus: string;
    onRefresh: () => void;
    table: Table<FriendLogRow>;
}) {
    const { t } = useTranslation();

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    <FriendLogTypeFilterDropdown
                        value={selectedTypes}
                        onChange={onSelectedTypesChange}
                    />
                </ToolbarViews>

                <ToolbarSearch
                    value={searchQuery}
                    onValueChange={onSearchQueryChange}
                    placeholder={t('view.friend_log.search_placeholder')}
                />

                <ToolbarActions>
                    <ToolbarRefreshButton
                        onRefresh={onRefresh}
                        loading={loadStatus === 'running'}
                        disabled={!currentUserId}
                    />
                    <TableColumnVisibilityMenu table={table} />
                </ToolbarActions>
            </PageToolbarRow>

            {detail ? <ToolbarStatus>{detail}</ToolbarStatus> : null}
        </PageToolbar>
    );
}
