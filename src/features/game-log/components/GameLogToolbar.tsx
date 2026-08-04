import type { Table as ReactTable } from '@tanstack/react-table';
import { LogsIcon, StarIcon, Table2Icon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { TableColumnVisibilityMenu } from '@/components/data-table/TableColumnVisibilityMenu';
import {
    DateTimeRangePicker,
    type DateTimeRangeValue
} from '@/components/date-time-range-picker/DateTimeRangePicker';
import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    toolbarDateRangeTrigger,
    ToolbarActions,
    ToolbarFilterChips,
    ToolbarRefreshButton,
    ToolbarSearch,
    ToolbarSegmented,
    ToolbarStatus,
    ToolbarToggleButton,
    ToolbarViews,
    type ToolbarSegmentOption
} from '@/components/layout/ToolbarControls';
import { formatCompactDateTime } from '@/lib/dateTime';

import { GAME_LOG_SESSION_DATE_RANGE_MAX_DAYS } from '../gameLogDateRange';
import type {
    GameLogFilterType,
    GameLogLoadStatus,
    GameLogRow,
    GameLogViewMode
} from '../gameLogTypes';

export function GameLogToolbar({
    detail,
    filterModel,
    refreshModel,
    table
}: {
    detail?: string;
    filterModel: {
        availableFilterTypes: readonly GameLogFilterType[];
        favoritesOnly: boolean;
        queryFilterTypes: readonly GameLogFilterType[];
        searchDraft: string;
        sessionDateRange: DateTimeRangeValue;
        todayDate: Date;
        viewMode: GameLogViewMode;
        changeViewMode(viewMode: GameLogViewMode): void;
        clearSearch(): void;
        commitSearchDraft(): void;
        setActiveSelectedTypes(types: GameLogFilterType[]): void;
        setSearchDraft(value: string): void;
        setSessionDateTimeRange(value: DateTimeRangeValue): void;
        toggleFavoritesOnly(): void;
    };
    refreshModel: {
        canRefresh: boolean;
        loadStatus: GameLogLoadStatus;
        onRefresh(): void;
    };
    table: ReactTable<GameLogRow>;
}) {
    const { t } = useTranslation();
    const {
        availableFilterTypes,
        favoritesOnly,
        queryFilterTypes,
        searchDraft,
        sessionDateRange,
        todayDate,
        viewMode,
        changeViewMode,
        clearSearch,
        commitSearchDraft,
        setActiveSelectedTypes,
        setSearchDraft,
        setSessionDateTimeRange,
        toggleFavoritesOnly
    } = filterModel;
    const { canRefresh, loadStatus, onRefresh } = refreshModel;
    const isTableView = viewMode === 'table';
    const viewModeOptions: ToolbarSegmentOption<GameLogViewMode>[] = [
        {
            value: 'sessions',
            label: t('view.game_log.label.sessions'),
            icon: LogsIcon
        },
        {
            value: 'table',
            label: t('view.game_log.label.table'),
            icon: Table2Icon
        }
    ];

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    <ToolbarSegmented
                        iconOnly
                        value={viewMode}
                        onValueChange={changeViewMode}
                        options={viewModeOptions}
                    />
                    <ToolbarToggleButton
                        icon={StarIcon}
                        fillWhenActive
                        active={favoritesOnly}
                        label={t('view.game_log.label.favorites_only')}
                        onClick={toggleFavoritesOnly}
                    />
                    <ToolbarFilterChips
                        value={queryFilterTypes}
                        allLabel={t('view.search.avatar.all')}
                        options={availableFilterTypes.map((type) => ({
                            value: type,
                            label: t(`view.game_log.filters.${type}`)
                        }))}
                        onValueChange={setActiveSelectedTypes}
                    />
                    {isTableView ? null : (
                        <DateTimeRangePicker
                            value={sessionDateRange}
                            onChange={setSessionDateTimeRange}
                            placeholder={t(
                                'view.game_log.label.session_date_range'
                            )}
                            startLabel={t('view.game_log.label.start')}
                            endLabel={t('view.game_log.label.end')}
                            clearLabel={t('common.actions.clear')}
                            confirmLabel={t('common.actions.confirm')}
                            formatValue={formatCompactDateTime}
                            maxDays={GAME_LOG_SESSION_DATE_RANGE_MAX_DAYS}
                            minuteStep={15}
                            align="start"
                            disabled={{ after: todayDate }}
                            renderTrigger={toolbarDateRangeTrigger}
                        />
                    )}
                </ToolbarViews>

                <ToolbarSearch
                    value={searchDraft}
                    onValueChange={setSearchDraft}
                    onCommit={commitSearchDraft}
                    onClear={clearSearch}
                />

                <ToolbarActions>
                    <ToolbarRefreshButton
                        onRefresh={onRefresh}
                        loading={loadStatus === 'running'}
                        disabled={!canRefresh}
                    />
                    {isTableView ? (
                        <TableColumnVisibilityMenu table={table} />
                    ) : null}
                </ToolbarActions>
            </PageToolbarRow>

            {detail ? <ToolbarStatus>{detail}</ToolbarStatus> : null}
        </PageToolbar>
    );
}
