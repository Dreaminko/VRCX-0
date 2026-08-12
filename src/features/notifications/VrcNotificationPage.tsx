import { InviteMessageDialog } from '@/components/dialogs/InviteMessageDialog';
import { PageScaffold } from '@/components/layout/PageScaffold';

import { NotificationFeed } from './components/NotificationFeed';
import { NotificationPageToolbar } from './components/NotificationPageToolbar';
import { BoopReplyDialog } from './components/NotificationViewParts';
import { useVrcNotificationPageController } from './useVrcNotificationPageController';

type VrcNotificationPageProps = {
    embedded?: boolean;
};

export function VrcNotificationPage({
    embedded = false
}: VrcNotificationPageProps = {}) {
    const {
        actions,
        dialogs,
        filters,
        handlers,
        notificationTypeLabel,
        pageRows,
        rowsState,
        runtime,
        sourceRowsCount,
        table,
        tableState,
        unseenCount
    } = useVrcNotificationPageController();

    return (
        <>
            <PageScaffold embedded={embedded} flushBottom={!embedded}>
                <NotificationPageToolbar
                    activeTypes={filters.activeTypes}
                    searchQuery={filters.searchQuery}
                    quickFilter={filters.quickFilter}
                    notificationTypeLabel={notificationTypeLabel}
                    loadStatus={rowsState.loadStatus}
                    unseenCount={unseenCount}
                    onActiveTypesChange={filters.setActiveTypes}
                    onSearchQueryChange={filters.setSearchQuery}
                    onQuickFilterChange={filters.setQuickFilter}
                    onMarkAllSeen={actions.markAllSeen}
                    onRefresh={rowsState.reload}
                    onClearFilters={filters.clearFilters}
                />
                <NotificationFeed
                    rows={pageRows}
                    table={table}
                    detail={rowsState.detail}
                    loadStatus={rowsState.loadStatus}
                    sourceRowsCount={sourceRowsCount}
                    hasActiveFilters={
                        filters.activeTypes.length > 0 ||
                        filters.quickFilter !== 'all' ||
                        Boolean(filters.searchQuery.trim())
                    }
                    rowsCount={rowsState.rows.length}
                    pagination={tableState.pagination}
                    pageSizes={tableState.pageSizes}
                    onPageSizeChange={tableState.handlePageSizeChange}
                    onClearFilters={filters.clearFilters}
                    currentUserId={runtime.currentUserId ?? undefined}
                    canInviteFromCurrentLocation={
                        runtime.canInviteFromCurrentLocation
                    }
                    handlers={handlers}
                />
            </PageScaffold>
            <InviteMessageDialog
                open={Boolean(dialogs.inviteResponseRequest)}
                onOpenChange={(open: boolean) => {
                    if (!open) {
                        dialogs.setInviteResponseRequest(null);
                    }
                }}
                currentUserId={runtime.currentUserId}
                endpoint={runtime.endpoint}
                messageType={
                    dialogs.inviteResponseRequest?.messageType || 'response'
                }
                mode="respond"
                targetLabel={String(
                    dialogs.inviteResponseRequest?.notification
                        ?.senderUsername ||
                        dialogs.inviteResponseRequest?.notification
                            ?.senderUserId ||
                        'this user'
                )}
                allowEdit
                allowImageUpload={runtime.isLocalUserVrcPlusSupporter}
                onUse={(payload) => {
                    const request = dialogs.inviteResponseRequest;
                    if (!request) {
                        return undefined;
                    }
                    return actions.sendInviteResponseSlot({
                        ...payload,
                        notification: request.notification
                    });
                }}
            />
            <BoopReplyDialog
                request={dialogs.boopReplyRequest}
                isLocalUserVrcPlusSupporter={
                    runtime.isLocalUserVrcPlusSupporter
                }
                onOpenChange={(open: boolean) => {
                    if (!open) {
                        dialogs.setBoopReplyRequest(null);
                    }
                }}
                onSend={actions.sendBoopReply}
            />
        </>
    );
}
