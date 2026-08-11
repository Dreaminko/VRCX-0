import { ChevronsUpDownIcon, ChevronUpIcon, UserRoundIcon } from 'lucide-react';
import type { ChangeEvent } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSearchParams } from 'react-router';
import { toast } from 'sonner';

import {
    DateTimeRangePicker,
    type DateTimeRangeValue
} from '@/components/date-time-range-picker/DateTimeRangePicker';
import {
    createdTime,
    rowLocation,
    rowMatchesSearch,
    sortPreviousInstanceRows
} from '@/components/dialogs/previous-instances-table/previousInstancesRows';
import { PreviousInstanceDetailsPanel } from '@/components/dialogs/previous-instances-table/PreviousInstancesViewParts';
import {
    PageBody,
    PageScaffold,
    PageToolbar,
    PageToolbarRow
} from '@/components/layout/PageScaffold';
import {
    toolbarDateRangeTrigger,
    ToolbarActions,
    ToolbarRefreshButton,
    ToolbarSearch,
    ToolbarSegmented,
    ToolbarStatus,
    ToolbarViewMenu,
    ToolbarViews
} from '@/components/layout/ToolbarControls';
import { UserPickerRow } from '@/components/search/UserPickerRow';
import { normalizeEndpoint, normalizeUserId } from '@/domain/users/userFacts';
import type { UserFact } from '@/domain/users/userFacts';
import { InstanceActivityDateControls } from '@/features/instance-history/components/InstanceActivityDateControls';
import { InstanceActivitySettingsPopover } from '@/features/instance-history/components/InstanceActivitySettingsPopover';
import { InstanceHistoryList } from '@/features/instance-history/components/InstanceHistoryList';
import {
    buildChartRows,
    buildDetailGroups,
    filterDetailGroups,
    getDetailGroupKeys
} from '@/features/instance-history/instance-activity/instanceActivityRows';
import type {
    InstanceActivityChartRow,
    PreviousInstanceRow
} from '@/features/instance-history/instance-activity/instanceActivityTypes';
import { useInstanceActivityChartLifecycle } from '@/features/instance-history/instance-activity/useInstanceActivityChartLifecycle';
import { useInstanceActivityData } from '@/features/instance-history/instance-activity/useInstanceActivityData';
import { useInstanceActivityRuntime } from '@/features/instance-history/instance-activity/useInstanceActivityRuntime';
import { useInstanceActivitySettings } from '@/features/instance-history/instance-activity/useInstanceActivitySettings';
import {
    buildLocalDayInstanceHistoryDateRange,
    emptyInstanceHistoryDateRange,
    isEmptyInstanceHistoryDateRange,
    refreshDefaultInstanceHistoryDateRange,
    resolveClearedInstanceHistoryDateRange,
    resolveScopedInstanceHistoryDateRange,
    type InstanceHistoryDateRangeState
} from '@/features/instance-history/instanceHistoryDateRange';
import {
    activityRowKey,
    buildAvailableInstanceHistoryDays,
    filterPreviousInstanceRowsForDay,
    findActivityRowForPreviousInstanceRow,
    findPreviousInstanceRowForActivityRow,
    sanitizeInstanceHistoryMode,
    selectDefaultInstanceHistoryDay
} from '@/features/instance-history/instanceHistoryDayMode';
import { formatCompactDateTime, timeToText } from '@/lib/dateTime';
import { cn } from '@/lib/utils';
import gameLogRepository from '@/repositories/gameLogRepository';
import { useModalStore } from '@/state/modalStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useUserFactsStore } from '@/state/userFactsStore';
import { Button } from '@/ui/shadcn/button';
import { Field, FieldContent, FieldGroup, FieldLabel } from '@/ui/shadcn/field';
import { Input } from '@/ui/shadcn/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/shadcn/popover';
import {
    ResizableHandle,
    ResizablePanel,
    ResizablePanelGroup
} from '@/ui/shadcn/resizable';
import { ScrollArea } from '@/ui/shadcn/scroll-area';
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';
import { Spinner } from '@/ui/shadcn/spinner';
import { Switch } from '@/ui/shadcn/switch';

type KnownUserOption = Partial<UserFact> & {
    id: string;
    endpoint: string;
    name?: string;
};

type TargetOption = {
    value: string;
    label: string;
    user: KnownUserOption;
};

type PreviousInstanceSortKey = 'date' | 'location' | 'duration';

const CHART_LOADING_INDICATOR_DELAY_MS = 150;

function knownUserName(user: Partial<KnownUserOption> | null | undefined) {
    return user?.displayName || user?.username || user?.name || '';
}

function dateRangeContains(
    row: PreviousInstanceRow,
    from: Date | null,
    to: Date | null
) {
    if (!from && !to) {
        return true;
    }
    const value = createdTime(row);
    if (!value) {
        return false;
    }
    if (from && value < from.getTime()) {
        return false;
    }
    if (to && value > to.getTime()) {
        return false;
    }
    return true;
}

export function InstanceHistoryPage({
    embedded = false
}: { embedded?: boolean } = {}) {
    const { t } = useTranslation();
    const [searchParams, setSearchParams] = useSearchParams();
    const confirm = useModalStore((state) => state.confirm);
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const currentUserDisplayName = useRuntimeStore(
        (state) => state.auth.currentUserDisplayName
    );
    const currentEndpoint = useRuntimeStore(
        (state) => state.auth.currentUserEndpoint
    );
    const usersByKey = useUserFactsStore((state) => state.usersByKey);
    const mode = sanitizeInstanceHistoryMode(searchParams.get('mode'));
    const isDayMode = mode === 'day';
    const [targetPickerOpen, setTargetPickerOpen] = useState(false);
    const [targetSearch, setTargetSearch] = useState('');
    const [rows, setRows] = useState<PreviousInstanceRow[]>([]);
    const [rowsQueryKey, setRowsQueryKey] = useState('');
    const [status, setStatus] = useState('idle');
    const [error, setError] = useState('');
    const [search, setSearch] = useState('');
    const [dateRangeState, setDateRangeState] =
        useState<InstanceHistoryDateRangeState>(() => ({
            range: emptyInstanceHistoryDateRange(),
            source: 'none'
        }));
    const [sortKey, setSortKey] = useState<PreviousInstanceSortKey>('date');
    const [sortDesc, setSortDesc] = useState(true);
    const [detailRow, setDetailRow] = useState<PreviousInstanceRow | null>(
        null
    );
    const [reloadToken, setReloadToken] = useState(0);
    const [selectedDay, setSelectedDay] = useState('');
    const [showChartLoadingIndicator, setShowChartLoadingIndicator] =
        useState(false);
    const targetSearchInputRef = useRef<HTMLInputElement>(null);
    const endpoint = normalizeEndpoint(currentEndpoint);
    const paramUserId = normalizeUserId(searchParams.get('id'));
    const paramSearch = searchParams.get('q') || '';
    const activeUserId = paramUserId || normalizeUserId(currentUserId);
    const isSelfScope = activeUserId === normalizeUserId(currentUserId);
    const historyScopeKey = `${endpoint}\u0000${activeUserId}\u0000${mode}\u0000${reloadToken}`;
    const scopedRows = rowsQueryKey.startsWith(`${historyScopeKey}\u0000`)
        ? rows
        : [];
    const dateRange = dateRangeState.range;
    const dateRangeSource = dateRangeState.source;
    const activityRuntime = useInstanceActivityRuntime(activeUserId);
    const activitySettings = useInstanceActivitySettings();
    const selectedDayForData = selectedDay || '';
    const activityData = useInstanceActivityData({
        currentEndpoint,
        currentUserId: isDayMode ? activeUserId : '',
        reloadToken,
        selectedDate: isDayMode ? selectedDayForData : ''
    });

    const knownUsers = useMemo(() => {
        const usersById = new Map<string, KnownUserOption>();
        if (currentUserId) {
            usersById.set(currentUserId, {
                id: currentUserId,
                displayName: currentUserDisplayName,
                endpoint
            });
        }
        for (const user of Object.values(usersByKey || {}).filter((user) => {
            const userId = normalizeUserId(user?.id);
            return (
                userId &&
                normalizeEndpoint(user?.endpoint || endpoint) === endpoint
            );
        })) {
            const userId = normalizeUserId(user?.id);
            if (!usersById.has(userId)) {
                usersById.set(userId, user);
            }
        }
        return Array.from(usersById.values())
            .sort((left, right) =>
                (knownUserName(left) || left?.id || '').localeCompare(
                    knownUserName(right) || right?.id || ''
                )
            )
            .slice(0, 500);
    }, [currentUserDisplayName, currentUserId, endpoint, usersByKey]);

    const activeKnownUser = useMemo<KnownUserOption | null>(
        () =>
            knownUsers.find(
                (user) => normalizeUserId(user?.id) === activeUserId
            ) || null,
        [activeUserId, knownUsers]
    );

    const activeUserLabel =
        (activeUserId && activeUserId === normalizeUserId(currentUserId)
            ? t('view.instance_history.label.self')
            : knownUserName(activeKnownUser)) ||
        (activeUserId === currentUserId ? currentUserDisplayName : '') ||
        t('view.instance_history.label.selected_user');

    const targetOptions = useMemo(() => {
        const query = targetSearch.trim().toLowerCase();
        return knownUsers
            .map((user): TargetOption => ({
                value: normalizeUserId(user?.id),
                label:
                    normalizeUserId(user?.id) === normalizeUserId(currentUserId)
                        ? t('view.instance_history.label.self')
                        : knownUserName(user) ||
                          t('view.instance_history.label.unnamed_user'),
                user
            }))
            .filter((option) => {
                if (!option.value) {
                    return false;
                }
                if (!query) {
                    return true;
                }
                return (
                    option.label.toLowerCase().includes(query) ||
                    option.value.toLowerCase().includes(query)
                );
            });
    }, [currentUserId, knownUsers, targetSearch, t]);

    const fallbackAvailableDays = useMemo(
        () => buildAvailableInstanceHistoryDays(scopedRows),
        [scopedRows]
    );
    const availableDays = activityData.availableDates.length
        ? activityData.availableDates
        : fallbackAvailableDays;
    const resolvedSelectedDay = selectDefaultInstanceHistoryDay(
        selectedDay,
        availableDays
    );
    const historyQueryDateRange = useMemo(
        () =>
            isDayMode
                ? buildLocalDayInstanceHistoryDateRange(resolvedSelectedDay)
                : dateRange,
        [dateRange, isDayMode, resolvedSelectedDay]
    );
    const isSearchDateRangeEmpty = isEmptyInstanceHistoryDateRange(dateRange);
    const isHistoryQueryDateRangeEmpty = isEmptyInstanceHistoryDateRange(
        historyQueryDateRange
    );
    const historyDateFrom = historyQueryDateRange.from?.toISOString() || '';
    const historyDateTo = historyQueryDateRange.to?.toISOString() || '';
    const historyQueryKey = `${historyScopeKey}\u0000${dateRangeSource}\u0000${historyDateFrom}\u0000${historyDateTo}`;
    const isDateRangeNormalizationPending =
        !isDayMode &&
        ((dateRangeSource === 'none' && isSearchDateRangeEmpty) ||
            (isSelfScope && dateRangeSource === 'unbounded'));
    const historyQueryReady =
        Boolean(activeUserId) &&
        !isDateRangeNormalizationPending &&
        !(isDayMode && isHistoryQueryDateRangeEmpty);
    const queryMatchesRows = rowsQueryKey === historyQueryKey;
    const visibleRows = queryMatchesRows ? rows : [];
    const visibleStatus = !historyQueryReady
        ? 'idle'
        : queryMatchesRows
          ? status
          : 'running';
    const visibleError = queryMatchesRows ? error : '';
    const visibleDetailRow = queryMatchesRows ? detailRow : null;
    const rawDayRows = useMemo(
        () =>
            filterPreviousInstanceRowsForDay(visibleRows, resolvedSelectedDay),
        [resolvedSelectedDay, visibleRows]
    );
    const rawChartRows = useMemo(
        () =>
            buildChartRows(
                activityData.rawRows,
                resolvedSelectedDay,
                activeUserId,
                activityData.worldDetailsById
            ),
        [
            activeUserId,
            activityData.rawRows,
            activityData.worldDetailsById,
            resolvedSelectedDay
        ]
    );
    const detailGroups = useMemo(
        () =>
            buildDetailGroups(
                activityData.rawRows,
                rawChartRows,
                activeUserId,
                activityRuntime.friendIdSet,
                activityRuntime.favoriteIdSet
            ),
        [
            activeUserId,
            activityData.rawRows,
            activityRuntime.favoriteIdSet,
            activityRuntime.friendIdSet,
            rawChartRows
        ]
    );
    const visibleDetailGroups = useMemo(
        () =>
            filterDetailGroups(detailGroups, {
                isSoloInstanceVisible: activitySettings.isSoloInstanceVisible,
                isNoFriendInstanceVisible:
                    activitySettings.isNoFriendInstanceVisible
            }),
        [
            activitySettings.isNoFriendInstanceVisible,
            activitySettings.isSoloInstanceVisible,
            detailGroups
        ]
    );
    const visibleActivityKeySet = useMemo(() => {
        const keys = new Set<string>();
        for (const group of visibleDetailGroups) {
            for (const key of getDetailGroupKeys(group, activeUserId)) {
                keys.add(key);
            }
        }
        return keys;
    }, [activeUserId, visibleDetailGroups]);
    const chartRows = useMemo(() => {
        if (activitySettings.isChartCollapsed || !rawChartRows.length) {
            return [];
        }
        if (!detailGroups.length) {
            return rawChartRows;
        }
        return rawChartRows.filter((row) =>
            visibleActivityKeySet.has(activityRowKey(row))
        );
    }, [
        activitySettings.isChartCollapsed,
        detailGroups.length,
        rawChartRows,
        visibleActivityKeySet
    ]);
    const totalOnlineTime = useMemo(
        () =>
            rawChartRows.reduce(
                (total, row) => total + row.visibleDurationMs,
                0
            ),
        [rawChartRows]
    );
    useEffect(() => {
        if (!paramSearch) {
            return;
        }
        setSearch(paramSearch);
        const nextParams = new URLSearchParams(searchParams);
        nextParams.delete('q');
        setSearchParams(nextParams, { replace: true });
    }, [paramSearch, searchParams, setSearchParams]);

    const [displayedOnlineTime, setDisplayedOnlineTime] = useState(0);
    useEffect(() => {
        if (activityData.dataStatus !== 'running') {
            setDisplayedOnlineTime(totalOnlineTime);
        }
    }, [activityData.dataStatus, totalOnlineTime]);
    const selectedActivityKey = visibleDetailRow
        ? findActivityRowForPreviousInstanceRow(visibleDetailRow, chartRows)
              ?.activityKey || ''
        : '';

    useEffect(() => {
        if (activityData.dataStatus !== 'running') {
            setShowChartLoadingIndicator(false);
            return undefined;
        }
        const timer = window.setTimeout(() => {
            setShowChartLoadingIndicator(true);
        }, CHART_LOADING_INDICATOR_DELAY_MS);
        return () => {
            window.clearTimeout(timer);
        };
    }, [activityData.dataStatus]);

    useEffect(() => {
        if (mode !== 'day') {
            return;
        }
        if (resolvedSelectedDay && resolvedSelectedDay !== selectedDay) {
            setSelectedDay(resolvedSelectedDay);
        }
    }, [mode, resolvedSelectedDay, selectedDay]);

    useEffect(() => {
        if (!activeUserId) {
            return;
        }
        setDateRangeState((currentState) =>
            resolveScopedInstanceHistoryDateRange({
                isDayMode,
                isSelfScope,
                state: currentState
            })
        );
    }, [activeUserId, isDayMode, isSelfScope]);

    useEffect(() => {
        if (!activeUserId) {
            setRows([]);
            setRowsQueryKey('');
            setStatus('idle');
            setError('');
            setDetailRow(null);
            return undefined;
        }
        if (!historyQueryReady) {
            setRows([]);
            setRowsQueryKey('');
            setStatus('idle');
            setError('');
            setDetailRow(null);
            return undefined;
        }

        let active = true;
        setRows([]);
        setRowsQueryKey(historyQueryKey);
        setStatus('running');
        setError('');
        setDetailRow(null);

        gameLogRepository
            .getPreviousInstancesByUserId(
                { id: activeUserId },
                {
                    dateFrom: historyDateFrom,
                    dateTo: historyDateTo
                }
            )
            .then((nextRows) => {
                if (!active) {
                    return;
                }
                setRows(nextRows);
                setStatus('ready');
            })
            .catch((loadError: unknown) => {
                if (!active) {
                    return;
                }
                setRows([]);
                setStatus('error');
                setError(
                    loadError instanceof Error
                        ? loadError.message
                        : t(
                              'view.instance_history.toast.failed_to_load_instance_history'
                          )
                );
            });

        return () => {
            active = false;
        };
    }, [
        activeUserId,
        historyDateFrom,
        historyDateTo,
        historyQueryKey,
        historyQueryReady,
        t
    ]);

    const filteredRows = useMemo(() => {
        const query = search.trim();
        const dateRows = visibleRows.filter((row) =>
            dateRangeContains(row, dateRange.from, dateRange.to)
        );
        const nextRows = query
            ? dateRows.filter((row) => rowMatchesSearch(row, query))
            : dateRows;
        return sortPreviousInstanceRows(nextRows, sortKey, sortDesc);
    }, [dateRange.from, dateRange.to, search, sortDesc, sortKey, visibleRows]);

    function selectSort(nextKey: PreviousInstanceSortKey, nextDesc: boolean) {
        setSortKey(nextKey);
        setSortDesc(Boolean(nextDesc));
    }

    function commitSearchParams({
        nextMode = mode,
        nextUserId = activeUserId
    }: {
        nextMode?: typeof mode;
        nextUserId?: string;
    }) {
        const params = new URLSearchParams();
        if (nextMode === 'day') {
            params.set('mode', 'day');
        }
        if (nextUserId && nextUserId !== normalizeUserId(currentUserId)) {
            params.set('scope', 'user');
            params.set('id', nextUserId);
        }
        setSearchParams(params);
    }

    function changeMode(nextMode: string) {
        const sanitizedMode = sanitizeInstanceHistoryMode(nextMode);
        commitSearchParams({ nextMode: sanitizedMode });
    }

    function applyTarget(value: string | null) {
        const nextUserId = normalizeUserId(value);
        if (!nextUserId) {
            return;
        }
        commitSearchParams({ nextUserId });
    }

    function refresh() {
        if (!activeUserId) {
            return;
        }
        setDateRangeState((currentState) =>
            refreshDefaultInstanceHistoryDateRange(currentState)
        );
        setReloadToken((value) => value + 1);
    }

    function clearDateRange() {
        setDateRangeState(
            resolveClearedInstanceHistoryDateRange({
                isDayMode,
                isSelfScope
            })
        );
    }

    function handleDateRangeChange(nextRange: DateTimeRangeValue) {
        if (isEmptyInstanceHistoryDateRange(nextRange)) {
            clearDateRange();
            return;
        }
        setDateRangeState({
            range: nextRange,
            source: 'user'
        });
    }

    function handleSearchChange(value: string) {
        setSearch(value);
    }

    const handleActivityRowActivate = useCallback(
        (activityRow: InstanceActivityChartRow) => {
            const matchedRow = findPreviousInstanceRowForActivityRow(
                activityRow,
                rawDayRows
            );
            if (matchedRow) {
                setDetailRow(matchedRow);
            }
        },
        [rawDayRows]
    );

    const activityChartLifecycle = useInstanceActivityChartLifecycle({
        barWidth: activitySettings.barWidth,
        chartRows,
        frozen: activityData.dataStatus === 'running',
        hour12: activityRuntime.hour12,
        onRowActivate: handleActivityRowActivate,
        resolvedTheme: activityRuntime.resolvedTheme,
        selectedActivityKey,
        selectedDate: resolvedSelectedDay
    });

    const dateActive = Boolean(dateRange.from || dateRange.to);

    const dateRangeLabel = dateActive
        ? [
              dateRange.from ? formatCompactDateTime(dateRange.from) : '...',
              dateRange.to ? formatCompactDateTime(dateRange.to) : '...'
          ].join(' - ')
        : t('view.instance_history.label.date_range');

    const sortItems: { value: PreviousInstanceSortKey; label: string }[] = [
        { value: 'date', label: t('table.previous_instances.date') },
        {
            value: 'location',
            label: t('dialog.previous_instances.label.location')
        },
        { value: 'duration', label: t('table.previous_instances.time') }
    ];

    const dateRangeControl = (
        <DateTimeRangePicker
            value={dateRange}
            onChange={handleDateRangeChange}
            align="start"
            renderTrigger={toolbarDateRangeTrigger}
            placeholder={t('view.instance_history.label.date_range')}
            startLabel={t('view.instance_history.label.start')}
            endLabel={t('view.instance_history.label.end')}
            clearLabel={t('common.actions.clear')}
            confirmLabel={t('common.actions.confirm')}
            formatValue={formatCompactDateTime}
            minuteStep={15}
            disabled={{ after: new Date() }}
        />
    );

    async function deleteRow(row: PreviousInstanceRow) {
        const location = rowLocation(row);
        if (!location || !activeUserId) {
            return;
        }
        const result = await confirm({
            title: t(
                'dialog.previous_instances_table.modal.delete_instance_record'
            ),
            description: location,
            destructive: true,
            confirmText: t('common.actions.delete'),
            cancelText: t('common.actions.cancel')
        });
        if (!result.ok) {
            return;
        }
        if (!Array.isArray(row.events) || row.events.length === 0) {
            toast.error(
                t(
                    'dialog.previous_instances.error.this_user_instance_row_cannot_be_deleted_without_event_ids'
                )
            );
            return;
        }
        try {
            await gameLogRepository.deleteGameLogInstance({
                id: activeUserId,
                location,
                events: row.events
            });
            setRows((currentRows) =>
                currentRows.filter((item) => item !== row)
            );
            setDetailRow((current) => (current === row ? null : current));
            if (isDayMode) {
                setReloadToken((value) => value + 1);
            }
            toast.success(
                t('dialog.previous_instances.success.instance_record_deleted')
            );
        } catch (deleteError) {
            toast.error(
                deleteError instanceof Error
                    ? deleteError.message
                    : t(
                          'dialog.previous_instances_table.toast.failed_to_delete_instance_record'
                      )
            );
        }
    }

    const listVisibleRows = isDayMode ? rawDayRows : filteredRows;
    const listTotalCount = isDayMode ? rawDayRows.length : visibleRows.length;
    const listFilteredCount = isDayMode
        ? rawDayRows.length
        : filteredRows.length;
    const dayStatus = activityData.dataStatus;
    const dayHasChartRows = chartRows.length > 0;
    const instanceHistoryListProps = {
        mode,
        totalCount: listTotalCount,
        filteredCount: listFilteredCount,
        visibleRows: listVisibleRows,
        selectedRow: visibleDetailRow,
        search,
        onSearchChange: handleSearchChange,
        sortKey,
        onOpenDetails: setDetailRow,
        onDeleteRow: deleteRow,
        dateActive,
        dateRangeLabel,
        onClearDate: clearDateRange
    };

    return (
        <PageScaffold embedded={embedded}>
            <PageToolbar>
                <PageToolbarRow>
                    <ToolbarViews>
                        <Popover
                            open={targetPickerOpen}
                            onOpenChange={setTargetPickerOpen}
                        >
                            <PopoverTrigger
                                render={
                                    <Button
                                        type="button"
                                        variant="outline"
                                        className="max-w-72 min-w-48 shrink-0 justify-between"
                                    >
                                        <UserRoundIcon
                                            data-icon="inline-start"
                                            className="text-muted-foreground"
                                        />
                                        <span className="min-w-0 flex-1 truncate text-left">
                                            {activeUserLabel}
                                        </span>
                                        <ChevronsUpDownIcon
                                            data-icon="inline-end"
                                            className="text-muted-foreground size-4"
                                        />
                                    </Button>
                                }
                            />
                            <PopoverContent
                                align="start"
                                className="w-96 p-2"
                                initialFocus={targetSearchInputRef}
                            >
                                <div className="flex flex-col gap-2">
                                    <Input
                                        ref={targetSearchInputRef}
                                        value={targetSearch}
                                        onChange={(
                                            event: ChangeEvent<HTMLInputElement>
                                        ) =>
                                            setTargetSearch(event.target.value)
                                        }
                                        placeholder={t(
                                            'view.instance_history.placeholder.user'
                                        )}
                                    />
                                    <ScrollArea className="h-72 rounded-md border">
                                        <div className="flex flex-col gap-1 p-1 pr-2">
                                            {targetOptions.map((option) => (
                                                <Button
                                                    key={option.value}
                                                    type="button"
                                                    variant="ghost"
                                                    className="h-auto justify-start p-0"
                                                    onClick={() => {
                                                        applyTarget(
                                                            option.value
                                                        );
                                                        setTargetPickerOpen(
                                                            false
                                                        );
                                                    }}
                                                >
                                                    <UserPickerRow
                                                        option={option}
                                                        selected={
                                                            option.value ===
                                                            activeUserId
                                                        }
                                                    />
                                                </Button>
                                            ))}
                                            {!targetOptions.length ? (
                                                <div className="text-muted-foreground p-3 text-xs">
                                                    {t(
                                                        'common.search_no_results'
                                                    )}
                                                </div>
                                            ) : null}
                                        </div>
                                    </ScrollArea>
                                </div>
                            </PopoverContent>
                        </Popover>
                        {!isSelfScope ? (
                            <Button
                                type="button"
                                variant="outline"
                                disabled={!currentUserId}
                                onClick={() => applyTarget(currentUserId)}
                            >
                                <UserRoundIcon data-icon="inline-start" />
                                {t('view.instance_history.action.current_user')}
                            </Button>
                        ) : null}
                        <ToolbarSegmented
                            value={mode}
                            onValueChange={changeMode}
                            options={[
                                {
                                    value: 'search',
                                    label: t(
                                        'view.instance_history.mode.search'
                                    )
                                },
                                {
                                    value: 'day',
                                    label: t('view.instance_history.mode.day')
                                }
                            ]}
                        />
                        {isDayMode ? null : dateRangeControl}
                    </ToolbarViews>

                    {isDayMode ? null : (
                        <ToolbarSearch
                            value={search}
                            onValueChange={setSearch}
                            placeholder={t(
                                'dialog.previous_instances.search_placeholder'
                            )}
                        />
                    )}

                    <ToolbarActions>
                        <ToolbarRefreshButton
                            onRefresh={refresh}
                            loading={visibleStatus === 'running'}
                            disabled={!activeUserId}
                        />
                        {isDayMode ? null : (
                            <ToolbarViewMenu contentClassName="p-3">
                                <FieldGroup
                                    onClick={(event) => event.stopPropagation()}
                                >
                                    <Field>
                                        <FieldContent>
                                            <FieldLabel>
                                                {t(
                                                    'dialog.previous_instances.label.sort_by'
                                                )}
                                            </FieldLabel>
                                        </FieldContent>
                                        <Select
                                            value={sortKey}
                                            items={sortItems}
                                            onValueChange={(value) =>
                                                selectSort(
                                                    (value ??
                                                        'date') as PreviousInstanceSortKey,
                                                    sortDesc
                                                )
                                            }
                                        >
                                            <SelectTrigger className="w-full">
                                                <SelectValue />
                                            </SelectTrigger>
                                            <SelectContent>
                                                <SelectGroup>
                                                    {sortItems.map((item) => (
                                                        <SelectItem
                                                            key={item.value}
                                                            value={item.value}
                                                        >
                                                            {item.label}
                                                        </SelectItem>
                                                    ))}
                                                </SelectGroup>
                                            </SelectContent>
                                        </Select>
                                    </Field>
                                    <Field orientation="horizontal">
                                        <FieldContent>
                                            <FieldLabel htmlFor="instance-history-sort-desc">
                                                {t(
                                                    'dialog.previous_instances.label.sort_descending'
                                                )}
                                            </FieldLabel>
                                        </FieldContent>
                                        <Switch
                                            id="instance-history-sort-desc"
                                            checked={sortDesc}
                                            onCheckedChange={(checked) =>
                                                selectSort(sortKey, checked)
                                            }
                                        />
                                    </Field>
                                </FieldGroup>
                            </ToolbarViewMenu>
                        )}
                    </ToolbarActions>
                </PageToolbarRow>
                {visibleStatus === 'error' ? (
                    <ToolbarStatus className="text-destructive">
                        {visibleError}
                    </ToolbarStatus>
                ) : null}
            </PageToolbar>
            <PageBody>
                <div className="flex min-h-0 flex-1 flex-col gap-3">
                    {isDayMode ? (
                        <div className="flex shrink-0 flex-col gap-3 rounded-md border p-3">
                            <div className="flex flex-wrap items-center justify-between gap-3">
                                <div className="flex flex-wrap items-center gap-3">
                                    <InstanceActivityDateControls
                                        selectedDate={resolvedSelectedDay}
                                        onSelectedDateChange={setSelectedDay}
                                        availableDates={availableDays}
                                        dataStatus={dayStatus}
                                    />
                                    <div className="flex items-baseline gap-2 text-sm">
                                        <span className="text-muted-foreground">
                                            {t(
                                                'view.charts.instance_activity.online_time'
                                            )}
                                        </span>
                                        <span className="font-medium tabular-nums">
                                            {timeToText(
                                                displayedOnlineTime,
                                                true
                                            )}
                                        </span>
                                    </div>
                                </div>
                                <div className="flex items-center gap-1">
                                    <InstanceActivitySettingsPopover
                                        barWidth={activitySettings.barWidth}
                                        isSoloInstanceVisible={
                                            activitySettings.isSoloInstanceVisible
                                        }
                                        isNoFriendInstanceVisible={
                                            activitySettings.isNoFriendInstanceVisible
                                        }
                                        onBarWidthCommit={
                                            activitySettings.handleBarWidthCommit
                                        }
                                        onSoloInstanceVisibleChange={
                                            activitySettings.setSoloInstanceVisible
                                        }
                                        onNoFriendInstanceVisibleChange={
                                            activitySettings.setNoFriendInstanceVisible
                                        }
                                    />
                                    <Button
                                        type="button"
                                        variant="ghost"
                                        size="icon-sm"
                                        aria-label={t(
                                            activitySettings.isChartCollapsed
                                                ? 'view.instance_history.day.expand_chart'
                                                : 'view.instance_history.day.collapse_chart'
                                        )}
                                        onClick={() =>
                                            activitySettings.setChartCollapsed(
                                                !activitySettings.isChartCollapsed
                                            )
                                        }
                                    >
                                        <ChevronUpIcon
                                            data-icon="icon"
                                            className={cn(
                                                'transition-transform duration-200 ease-out',
                                                activitySettings.isChartCollapsed &&
                                                    'rotate-180'
                                            )}
                                        />
                                    </Button>
                                </div>
                            </div>
                            {activityData.availableDatesStatus === 'error' ? (
                                <div className="text-destructive text-sm">
                                    {activityData.availableDatesError ||
                                        t(
                                            'view.charts.error.instance_activity_failed_to_load'
                                        )}
                                </div>
                            ) : null}
                            {activitySettings.isChartCollapsed ? null : dayStatus ===
                              'error' ? (
                                <div className="text-destructive text-sm">
                                    {activityData.dataDetail ||
                                        t(
                                            'view.charts.error.instance_activity_failed_to_load'
                                        )}
                                </div>
                            ) : (
                                <div className="relative">
                                    <div
                                        ref={
                                            activityChartLifecycle.setMainChartElementRef
                                        }
                                        className={cn(
                                            'min-h-24 w-full bg-transparent',
                                            dayStatus === 'running' &&
                                                'pointer-events-none opacity-60'
                                        )}
                                    />
                                    {dayStatus === 'running' &&
                                    showChartLoadingIndicator ? (
                                        <div className="text-muted-foreground pointer-events-none absolute inset-0 flex items-center justify-center gap-2 text-sm">
                                            <Spinner className="size-4" />
                                            {t(
                                                'view.charts.loading.loading_instance_activity'
                                            )}
                                        </div>
                                    ) : null}
                                    {dayStatus !== 'running' &&
                                    !dayHasChartRows ? (
                                        <div className="text-muted-foreground text-sm">
                                            {t(
                                                'view.charts.empty.no_instance_activity_on_this_day'
                                            )}
                                        </div>
                                    ) : null}
                                </div>
                            )}
                        </div>
                    ) : null}
                    <ResizablePanelGroup
                        id="instance-history-layout"
                        orientation="horizontal"
                        className="min-h-0 flex-1"
                    >
                        <ResizablePanel
                            id="instance-history-list"
                            defaultSize={36}
                            minSize={28}
                            className="min-h-0 min-w-0 pr-3"
                        >
                            <InstanceHistoryList
                                {...instanceHistoryListProps}
                            />
                        </ResizablePanel>
                        <ResizableHandle withHandle />
                        <ResizablePanel
                            id="instance-history-details"
                            defaultSize={64}
                            minSize={40}
                            className="min-h-0 min-w-0 pl-3"
                        >
                            <PreviousInstanceDetailsPanel
                                row={visibleDetailRow}
                                showTitle
                                className="h-full min-h-0"
                            />
                        </ResizablePanel>
                    </ResizablePanelGroup>
                </div>
            </PageBody>
        </PageScaffold>
    );
}
