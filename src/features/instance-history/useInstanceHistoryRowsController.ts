import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { rowLocation } from '@/components/dialogs/previous-instances-table/previousInstancesRows';
import gameLogRepository from '@/repositories/gameLogRepository';
import { useModalStore } from '@/state/modalStore';

import type { PreviousInstanceRow } from './instance-activity/instanceActivityTypes';
import {
    buildLocalDayInstanceHistoryDateRange,
    isEmptyInstanceHistoryDateRange,
    type InstanceHistoryDateRangeState
} from './instanceHistoryDateRange';
import {
    buildAvailableInstanceHistoryDays,
    sanitizeInstanceHistoryMode,
    selectDefaultInstanceHistoryDay
} from './instanceHistoryDayMode';

export function useInstanceHistoryRowsController({
    activeUserId,
    availableActivityDates,
    dateRangeState,
    endpoint,
    isSelfScope,
    mode,
    reloadDayData,
    reloadToken,
    selectedDay
}: {
    activeUserId: string;
    availableActivityDates: string[];
    dateRangeState: InstanceHistoryDateRangeState;
    endpoint: string;
    isSelfScope: boolean;
    mode: ReturnType<typeof sanitizeInstanceHistoryMode>;
    reloadDayData: () => void;
    reloadToken: number;
    selectedDay: string;
}) {
    const { t } = useTranslation();
    const confirm = useModalStore((state) => state.confirm);
    const [rows, setRows] = useState<PreviousInstanceRow[]>([]);
    const [rowsQueryKey, setRowsQueryKey] = useState('');
    const [status, setStatus] = useState('idle');
    const [error, setError] = useState('');
    const [detailRow, setDetailRow] = useState<PreviousInstanceRow | null>(
        null
    );
    const isDayMode = mode === 'day';
    const historyScopeKey = `${endpoint}\u0000${activeUserId}\u0000${mode}\u0000${reloadToken}`;
    const fallbackAvailableDays = useMemo(
        () =>
            buildAvailableInstanceHistoryDays(
                rowsQueryKey.startsWith(`${historyScopeKey}\u0000`) ? rows : []
            ),
        [historyScopeKey, rows, rowsQueryKey]
    );
    const availableDays = availableActivityDates.length
        ? availableActivityDates
        : fallbackAvailableDays;
    const resolvedSelectedDay = selectDefaultInstanceHistoryDay(
        selectedDay,
        availableDays
    );
    const historyQueryDateRange = useMemo(
        () =>
            isDayMode
                ? buildLocalDayInstanceHistoryDateRange(resolvedSelectedDay)
                : dateRangeState.range,
        [dateRangeState.range, isDayMode, resolvedSelectedDay]
    );
    const isSearchDateRangeEmpty = isEmptyInstanceHistoryDateRange(
        dateRangeState.range
    );
    const isHistoryQueryDateRangeEmpty = isEmptyInstanceHistoryDateRange(
        historyQueryDateRange
    );
    const historyDateFrom = historyQueryDateRange.from?.toISOString() || '';
    const historyDateTo = historyQueryDateRange.to?.toISOString() || '';
    const historyQueryKey = `${historyScopeKey}\u0000${dateRangeState.source}\u0000${historyDateFrom}\u0000${historyDateTo}`;
    const isDateRangeNormalizationPending =
        !isDayMode &&
        ((dateRangeState.source === 'none' && isSearchDateRangeEmpty) ||
            (isSelfScope && dateRangeState.source === 'unbounded'));
    const historyQueryReady =
        Boolean(activeUserId) &&
        !isDateRangeNormalizationPending &&
        !(isDayMode && isHistoryQueryDateRangeEmpty);

    useEffect(() => {
        if (!activeUserId || !historyQueryReady) {
            setRows([]);
            setRowsQueryKey('');
            setStatus('idle');
            setError('');
            setDetailRow(null);
            return;
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
                { dateFrom: historyDateFrom, dateTo: historyDateTo }
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
                reloadDayData();
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

    const queryMatchesRows = rowsQueryKey === historyQueryKey;
    return {
        availableDays,
        deleteRow,
        detailRow: queryMatchesRows ? detailRow : null,
        error: queryMatchesRows ? error : '',
        resolvedSelectedDay,
        rows: queryMatchesRows ? rows : [],
        setDetailRow,
        status: !historyQueryReady
            ? 'idle'
            : queryMatchesRows
              ? status
              : 'running'
    };
}
