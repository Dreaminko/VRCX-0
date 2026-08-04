import { StarIcon } from 'lucide-react';
import { memo, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    toolbarDateRangeTrigger,
    ToolbarActions,
    ToolbarFilterChips,
    ToolbarSearch,
    ToolbarToggleButton,
    ToolbarViews
} from '@/components/layout/ToolbarControls';
import type { FeedFilterType } from '@/repositories/feedRepository';
import { Button } from '@/ui/shadcn/button';
import { Calendar } from '@/ui/shadcn/calendar';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/shadcn/popover';

import type { FeedDateRange } from '../feedTypes';

type FeedToolbarProps = {
    columnsMenu: ReactNode;
    filterCommands: {
        onApplyDateFilter(): void;
        onClearDateFilter(): void;
        onClearFeedFilters(): void;
        onClearSearch(): void;
        onCommitSearch(): void;
        onDateFilterOpenChange(open: boolean): void;
        onDateRangeSelect(range?: FeedDateRange): void;
        onSearchDraftChange(value: string): void;
        onToggleFavoritesOnly(): void;
        onToggleFeedFilter(filter: FeedFilterType): void;
    };
    filterModel: {
        activeFilters: FeedFilterType[];
        dateDraftFrom: string;
        dateDraftRange?: FeedDateRange;
        dateDraftTo: string;
        dateFilterOpen: boolean;
        dateFrom: string;
        dateTo: string;
        favoritesOnly: boolean;
        feedFilterTypes: readonly FeedFilterType[];
        searchDraft: string;
        todayDate: Date;
    };
    modeToggle: ReactNode;
};

function FeedTypeFilterChips({
    activeFilters,
    feedFilterTypes,
    onClearFeedFilters,
    onToggleFeedFilter
}: {
    activeFilters: FeedFilterType[];
    feedFilterTypes: readonly FeedFilterType[];
    onClearFeedFilters(): void;
    onToggleFeedFilter(filter: FeedFilterType): void;
}) {
    const { t } = useTranslation();

    return (
        <ToolbarFilterChips
            value={activeFilters}
            allLabel={t('view.search.avatar.all')}
            options={feedFilterTypes.map((filter) => ({
                value: filter,
                label: t(`view.feed.filters.${filter}`)
            }))}
            onValueChange={(next) => {
                if (!next.length) {
                    onClearFeedFilters();
                    return;
                }
                const current = new Set(activeFilters);
                const wanted = new Set(next);
                feedFilterTypes.forEach((filter) => {
                    if (current.has(filter) !== wanted.has(filter)) {
                        onToggleFeedFilter(filter);
                    }
                });
            }}
        />
    );
}

function FeedDateRangeFilter({
    dateDraftFrom,
    dateDraftRange,
    dateDraftTo,
    dateFilterOpen,
    dateFrom,
    dateTo,
    onApplyDateFilter,
    onClearDateFilter,
    onDateFilterOpenChange,
    onDateRangeSelect,
    todayDate
}: {
    dateDraftFrom: string;
    dateDraftRange?: FeedDateRange;
    dateDraftTo: string;
    dateFilterOpen: boolean;
    dateFrom: string;
    dateTo: string;
    onApplyDateFilter(): void;
    onClearDateFilter(): void;
    onDateFilterOpenChange(open: boolean): void;
    onDateRangeSelect(range?: FeedDateRange): void;
    todayDate: Date;
}) {
    const { t } = useTranslation();
    const hasRange = Boolean(dateFrom || dateTo);
    const label = hasRange
        ? [dateFrom || '...', dateTo || '...'].join(' - ')
        : t('view.feed.date_range');

    return (
        <Popover open={dateFilterOpen} onOpenChange={onDateFilterOpenChange}>
            <PopoverTrigger
                render={toolbarDateRangeTrigger({ active: hasRange, label })}
            />
            <PopoverContent className="w-auto" align="start">
                <Calendar
                    mode="range"
                    numberOfMonths={2}
                    selected={dateDraftRange}
                    disabled={{ after: todayDate }}
                    onSelect={onDateRangeSelect}
                />
                <div className="flex items-center justify-between gap-4 px-3 pb-3">
                    <div className="text-muted-foreground min-w-0 text-xs">
                        {[dateDraftFrom || '...', dateDraftTo || '...'].join(
                            ' - '
                        )}
                    </div>
                    <div className="flex justify-end gap-2">
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={onClearDateFilter}
                        >
                            {t('common.actions.clear')}
                        </Button>
                        <Button
                            type="button"
                            size="sm"
                            onClick={onApplyDateFilter}
                        >
                            {t('common.actions.confirm')}
                        </Button>
                    </div>
                </div>
            </PopoverContent>
        </Popover>
    );
}

export const FeedToolbar = memo(function FeedToolbar({
    columnsMenu,
    filterCommands,
    filterModel,
    modeToggle
}: FeedToolbarProps) {
    const { t } = useTranslation();
    const {
        activeFilters,
        dateDraftFrom,
        dateDraftRange,
        dateDraftTo,
        dateFilterOpen,
        dateFrom,
        dateTo,
        favoritesOnly,
        feedFilterTypes,
        searchDraft,
        todayDate
    } = filterModel;
    const {
        onApplyDateFilter,
        onClearDateFilter,
        onClearFeedFilters,
        onClearSearch,
        onCommitSearch,
        onDateFilterOpenChange,
        onDateRangeSelect,
        onSearchDraftChange,
        onToggleFavoritesOnly,
        onToggleFeedFilter
    } = filterCommands;

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    {modeToggle}
                    <ToolbarToggleButton
                        icon={StarIcon}
                        fillWhenActive
                        active={favoritesOnly}
                        label={t('view.feed.favorites_only_tooltip')}
                        onClick={onToggleFavoritesOnly}
                    />
                    <FeedTypeFilterChips
                        activeFilters={activeFilters}
                        feedFilterTypes={feedFilterTypes}
                        onClearFeedFilters={onClearFeedFilters}
                        onToggleFeedFilter={onToggleFeedFilter}
                    />
                    <FeedDateRangeFilter
                        dateDraftFrom={dateDraftFrom}
                        dateDraftRange={dateDraftRange}
                        dateDraftTo={dateDraftTo}
                        dateFilterOpen={dateFilterOpen}
                        dateFrom={dateFrom}
                        dateTo={dateTo}
                        onApplyDateFilter={onApplyDateFilter}
                        onClearDateFilter={onClearDateFilter}
                        onDateFilterOpenChange={onDateFilterOpenChange}
                        onDateRangeSelect={onDateRangeSelect}
                        todayDate={todayDate}
                    />
                </ToolbarViews>

                <ToolbarSearch
                    value={searchDraft}
                    onValueChange={onSearchDraftChange}
                    onCommit={onCommitSearch}
                    onClear={onClearSearch}
                    placeholder={t('view.feed.search_placeholder')}
                />

                <ToolbarActions>{columnsMenu}</ToolbarActions>
            </PageToolbarRow>
        </PageToolbar>
    );
});
