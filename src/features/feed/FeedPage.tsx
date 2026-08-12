import { Columns3Icon, TableIcon } from 'lucide-react';
import { useCallback, useMemo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { useSearchParams } from 'react-router';

import { TableColumnVisibilityMenu } from '@/components/data-table/TableColumnVisibilityMenu';
import { PreviousInstancesTableDialog } from '@/components/dialogs/PreviousInstancesTableDialog';
import { PageBody, PageScaffold } from '@/components/layout/PageScaffold';
import {
    ToolbarSegmented,
    type ToolbarSegmentOption
} from '@/components/layout/ToolbarControls';
import { usePreferencesStore } from '@/state/preferencesStore';
import { Spinner } from '@/ui/shadcn/spinner';

import { FeedColumnsMode } from './columns/FeedColumnsMode';
import { FeedTableShell } from './components/FeedTableShell';
import { FeedToolbar } from './components/FeedToolbar';
import type { FeedViewMode } from './feedColumnsState';
import { readFeedRouteUserIds, withFeedRouteUserIds } from './feedRouteScope';
import { useFeedPageController } from './useFeedPageController';
import { useFeedRowArrivals } from './useFeedRowArrivals';
import { useFeedViewModeState } from './useFeedViewModeState';

type FeedPageProps = {
    embedded?: boolean;
};

function FeedViewModeToggle({
    onValueChange,
    value
}: {
    onValueChange(value: FeedViewMode): void;
    value: FeedViewMode;
}) {
    const { t } = useTranslation();
    const options: ToolbarSegmentOption<FeedViewMode>[] = [
        {
            value: 'table',
            label: t('view.feed.modes.table'),
            icon: TableIcon
        },
        {
            value: 'columns',
            label: t('view.feed.modes.columns'),
            icon: Columns3Icon
        }
    ];

    return (
        <ToolbarSegmented
            iconOnly
            value={value}
            onValueChange={onValueChange}
            options={options}
        />
    );
}

export function FeedPage({ embedded = false }: FeedPageProps = {}) {
    const [searchParams, setSearchParams] = useSearchParams();
    const routeScopedUserIds = useMemo(
        () => (embedded ? [] : readFeedRouteUserIds(searchParams)),
        [embedded, searchParams]
    );
    const {
        columns,
        density,
        ready,
        setColumns,
        setDensity,
        setViewMode,
        viewMode
    } = useFeedViewModeState();
    const effectiveViewMode =
        !embedded && searchParams.get('feedView') === 'table'
            ? 'table'
            : viewMode;
    const setEffectiveViewMode = useCallback(
        (value: FeedViewMode) => {
            if (!embedded && searchParams.has('feedView')) {
                const nextSearchParams = new URLSearchParams(searchParams);
                nextSearchParams.delete('feedView');
                setSearchParams(nextSearchParams, { replace: true });
            }
            setViewMode(value);
        },
        [embedded, searchParams, setSearchParams, setViewMode]
    );
    const modeToggle = (
        <FeedViewModeToggle
            value={effectiveViewMode}
            onValueChange={setEffectiveViewMode}
        />
    );
    const feedPersistenceDisabled = usePreferencesStore(
        (state) => state.feedPersistenceDisabled
    );
    const setRouteScopedUserIds = useCallback(
        (userIds: readonly string[]) => {
            if (embedded) {
                return;
            }
            setSearchParams(withFeedRouteUserIds(searchParams, userIds), {
                replace: true
            });
        },
        [embedded, searchParams, setSearchParams]
    );

    if (!ready) {
        return (
            <PageScaffold
                embedded={embedded}
                className={embedded ? '' : 'feed'}
            >
                <PageBody className="items-center justify-center">
                    <Spinner />
                </PageBody>
            </PageScaffold>
        );
    }

    return (
        <PageScaffold embedded={embedded} className={embedded ? '' : 'feed'}>
            {effectiveViewMode === 'columns' ? (
                <PageBody className="gap-2">
                    <FeedColumnsMode
                        columns={columns}
                        density={density}
                        modeToggle={modeToggle}
                        onColumnsChange={setColumns}
                        onDensityChange={setDensity}
                        feedPersistenceDisabled={feedPersistenceDisabled}
                    />
                </PageBody>
            ) : (
                <FeedTableMode
                    modeToggle={modeToggle}
                    feedPersistenceDisabled={feedPersistenceDisabled}
                    routeScopedUserIds={routeScopedUserIds}
                    setRouteScopedUserIds={setRouteScopedUserIds}
                />
            )}
        </PageScaffold>
    );
}

function FeedTableMode({
    modeToggle,
    feedPersistenceDisabled,
    routeScopedUserIds,
    setRouteScopedUserIds
}: {
    modeToggle: ReactNode;
    feedPersistenceDisabled: boolean;
    routeScopedUserIds: readonly string[];
    setRouteScopedUserIds(userIds: readonly string[]): void;
}) {
    const {
        columns,
        filters,
        friendActions,
        isFavoritesLoaded,
        loadStatus,
        previousInstancesDialog,
        resolvePageSize,
        rows,
        table,
        tableModel
    } = useFeedPageController({ routeScopedUserIds });
    const arrivals = useFeedRowArrivals(rows, loadStatus);
    const {
        activeFilters,
        applyDateFilter,
        clearDateFilter,
        clearSearch,
        commitSearch,
        dateDraftFrom,
        dateDraftRange,
        dateDraftTo,
        dateFilterOpen,
        dateFrom,
        dateTo,
        favoritesOnly,
        feedFilterTypes,
        onDateRangeSelect,
        scopedUserIds,
        searchDraft,
        setDateFilterOpen,
        setFavoritesOnly,
        setFeedFilters,
        setSearchDraft,
        setUserScope,
        todayDate,
        toggleFeedFilter
    } = filters;
    const isSearching =
        loadStatus === 'running' &&
        Boolean(
            filters.deferredSearchQuery.trim() ||
            filters.deferredScopedUserIds.length
        );
    const columnsMenu = <TableColumnVisibilityMenu table={table} />;
    const filterModel = useMemo(
        () => ({
            activeFilters,
            dateDraftFrom,
            dateDraftRange,
            dateDraftTo,
            dateFilterOpen,
            dateFrom,
            dateTo,
            favoritesOnly,
            feedFilterTypes,
            scopedUserIds,
            searchDraft,
            todayDate
        }),
        [
            activeFilters,
            dateDraftFrom,
            dateDraftRange,
            dateDraftTo,
            dateFilterOpen,
            dateFrom,
            dateTo,
            favoritesOnly,
            feedFilterTypes,
            scopedUserIds,
            searchDraft,
            todayDate
        ]
    );
    const filterCommands = useMemo(
        () => ({
            onApplyDateFilter: applyDateFilter,
            onClearDateFilter: clearDateFilter,
            onClearFeedFilters: () => setFeedFilters([]),
            onClearSearch: clearSearch,
            onCommitSearch: () => commitSearch(),
            onDateFilterOpenChange: setDateFilterOpen,
            onDateRangeSelect,
            onScopeChange: (userIds: readonly string[]) => {
                setUserScope(userIds);
                setRouteScopedUserIds(userIds);
            },
            onSearchDraftChange: setSearchDraft,
            onToggleFavoritesOnly: () =>
                setFavoritesOnly((current) => !current),
            onToggleFeedFilter: toggleFeedFilter
        }),
        [
            applyDateFilter,
            clearDateFilter,
            clearSearch,
            commitSearch,
            onDateRangeSelect,
            setDateFilterOpen,
            setFavoritesOnly,
            setFeedFilters,
            setRouteScopedUserIds,
            setSearchDraft,
            setUserScope,
            toggleFeedFilter
        ]
    );

    return (
        <>
            <FeedToolbar
                columnsMenu={columnsMenu}
                filterModel={filterModel}
                filterCommands={filterCommands}
                modeToggle={modeToggle}
                feedPersistenceDisabled={feedPersistenceDisabled}
                isSearching={isSearching}
            />
            <PageBody>
                <FeedTableShell
                    arrivals={arrivals}
                    columns={columns}
                    favoritesOnly={filters.favoritesOnly}
                    isFavoritesLoaded={isFavoritesLoaded}
                    loadStatus={loadStatus}
                    loadingPreviousInstancesKey={
                        previousInstancesDialog.loadingKey
                    }
                    onNewInstance={friendActions.openFeedNewInstance}
                    onOpenPreviousInstances={
                        previousInstancesDialog.openPreviousInstancesForLocation
                    }
                    onPaginationChange={tableModel.setPagination}
                    pageSizes={tableModel.pageSizes}
                    pagination={tableModel.pagination}
                    resolvePageSize={resolvePageSize}
                    rows={rows}
                    table={table}
                />
            </PageBody>
            <PreviousInstancesTableDialog
                open={previousInstancesDialog.open}
                onOpenChange={previousInstancesDialog.setOpen}
                title={previousInstancesDialog.title}
                instances={previousInstancesDialog.rows}
                onRowsChange={previousInstancesDialog.setRows}
            />
        </>
    );
}
