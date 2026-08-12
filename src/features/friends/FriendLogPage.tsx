import { HistoryIcon, SearchXIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import {
    LoadingState,
    PageBody,
    PageScaffold
} from '@/components/layout/PageScaffold';
import { Button } from '@/ui/shadcn/button';

import { FriendLogPageTable } from './components/FriendLogPageTable';
import { FriendLogPageToolbar } from './components/FriendLogPageToolbar';
import { FriendLogEmptyState } from './components/FriendLogViewParts';
import { useFriendLogPageController } from './useFriendLogPageController';

export function FriendLogPage({
    embedded = false
}: { embedded?: boolean } = {}) {
    const { t } = useTranslation();
    const { filters, isError, isLoading, rows, table, tableState } =
        useFriendLogPageController();
    const hasRows = rows.orderedRows.length > 0;
    const hasHistory = rows.rows.length > 0;

    return (
        <PageScaffold embedded={embedded}>
            <FriendLogPageToolbar
                selectedTypes={filters.selectedTypes}
                onSelectedTypesChange={filters.setSelectedTypes}
                searchQuery={filters.searchQuery}
                onSearchQueryChange={filters.setSearchQuery}
                detail={rows.detail}
                currentUserId={rows.currentUserId}
                loadStatus={rows.loadStatus}
                onRefresh={filters.refreshFriendLog}
                table={table}
            />

            <PageBody>
                {isLoading ? (
                    <LoadingState
                        label={t(
                            'view.friend_log.loading.loading_the_friend_history_snapshot'
                        )}
                    />
                ) : isError ? (
                    <FriendLogEmptyState
                        title={t(
                            'view.friend_log.error.friend_history_failed_to_load'
                        )}
                        description={
                            rows.detail || 'The history query did not complete.'
                        }
                    />
                ) : hasRows ? (
                    <FriendLogPageTable
                        table={table}
                        orderedRowsLength={rows.orderedRows.length}
                        pagination={tableState.pagination}
                        pageSizes={tableState.pageSizes}
                        onPageSizeChange={tableState.setPageSize}
                    />
                ) : (
                    <FriendLogEmptyState
                        icon={hasHistory ? SearchXIcon : HistoryIcon}
                        title={t(
                            hasHistory
                                ? 'view.friend_log.empty.no_friend_history_rows_match_the_current_filters'
                                : 'empty_state.friend_history_title'
                        )}
                        description={t(
                            hasHistory
                                ? 'view.friend_log.label.broaden_the_type_filters_or_search_query_to_see_more_history'
                                : 'empty_state.friend_history_description'
                        )}
                    >
                        {hasHistory ? (
                            <Button
                                type="button"
                                variant="link"
                                onClick={() => {
                                    filters.setSelectedTypes([]);
                                    filters.setSearchQuery('');
                                }}
                            >
                                {t('common.actions.clear')}
                            </Button>
                        ) : null}
                    </FriendLogEmptyState>
                )}
            </PageBody>
        </PageScaffold>
    );
}
