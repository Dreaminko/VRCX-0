import type { Column, Row } from '@tanstack/react-table';
import { ChevronRightIcon } from 'lucide-react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

import { useKnownUserFacts } from '@/lib/useKnownUser';
import { cn } from '@/lib/utils';
import { Button } from '@/ui/shadcn/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/ui/shadcn/tooltip';

import { resolveFeedUserDisplayName, resolveFeedUserId } from '../feedRows';
import type {
    FeedColumns,
    FeedFriendActions,
    FeedLocationActionPayload,
    FeedRow,
    FeedTableInstance
} from '../feedTypes';
import {
    FeedDetailCell,
    FeedUserLink,
    SortButton,
    formatTimestampLong,
    formatTimestampParts
} from './FeedTableParts';
import { FeedTypeIndicator } from './FeedTypeIndicator';

type UseFeedColumnsOptions = {
    actions: FeedFriendActions;
    friendLogNamesById: Record<string, string>;
    loadingPreviousInstancesKey: string;
    onOpenPreviousInstances(payload?: FeedLocationActionPayload): void;
    rows: FeedRow[];
};

function ExpanderCell({ row }: { row: Row<FeedRow> }) {
    if (!row.getCanExpand()) {
        return null;
    }

    return (
        <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            onClick={() => row.toggleExpanded()}
        >
            <ChevronRightIcon
                data-icon="icon"
                className={cn(
                    'transition-transform duration-150 ease-out',
                    row.getIsExpanded() && 'rotate-90'
                )}
            />
        </Button>
    );
}

function DateCell({ row }: { row: Row<FeedRow> }) {
    const { date, time } = formatTimestampParts(row.original.created_at);
    return (
        <Tooltip>
            <TooltipTrigger
                render={
                    <span className="text-sm">
                        <span className="text-muted-foreground">{date}</span>
                        {time ? (
                            <span className="text-foreground ml-1">{time}</span>
                        ) : null}
                    </span>
                }
            />
            <TooltipContent side="right">
                {formatTimestampLong(row.original.created_at)}
            </TooltipContent>
        </Tooltip>
    );
}

function resolveDisplayNameCellClassName(
    row: Row<FeedRow>,
    table: FeedTableInstance
) {
    const currentUserId = resolveFeedUserId(row.original);
    if (!currentUserId) {
        return '';
    }

    const visibleRows = table.getRowModel().rows;
    const rowIndex = visibleRows.findIndex(
        (visibleRow) => visibleRow.id === row.id
    );
    if (rowIndex <= 0) {
        return '';
    }

    const previousRow = visibleRows[rowIndex - 1];
    const previousUserId = resolveFeedUserId(previousRow.original);
    return previousUserId === currentUserId ? 'text-muted-foreground' : '';
}

export function useFeedColumns({
    actions,
    friendLogNamesById,
    loadingPreviousInstancesKey,
    onOpenPreviousInstances,
    rows
}: UseFeedColumnsOptions): FeedColumns {
    const { t } = useTranslation();
    const rowUserIds = useMemo(
        () => rows.map(resolveFeedUserId).filter(Boolean),
        [rows]
    );
    const knownUsersById = useKnownUserFacts(rowUserIds);

    return useMemo(
        () => [
            {
                id: 'expander',
                size: 20,
                enableSorting: false,
                enableHiding: false,
                meta: { label: '' },
                header: () => null,
                cell: ({ row }: { row: Row<FeedRow> }) => (
                    <ExpanderCell row={row} />
                )
            },
            {
                id: 'created_at',
                accessorFn: (row: FeedRow) =>
                    new Date(String(row?.created_at || 0)).valueOf() || 0,
                meta: { label: t('table.feed.date') },
                header: ({ column }: { column: Column<FeedRow, unknown> }) => (
                    <SortButton column={column} label={t('table.feed.date')} />
                ),
                cell: ({ row }: { row: Row<FeedRow> }) => <DateCell row={row} />
            },
            {
                id: 'type',
                accessorFn: (row: FeedRow) => String(row?.type || ''),
                meta: { label: t('table.feed.type') },
                header: ({ column }: { column: Column<FeedRow, unknown> }) => (
                    <SortButton column={column} label={t('table.feed.type')} />
                ),
                cell: ({ row }: { row: Row<FeedRow> }) => {
                    const typeLabel = row.original.type
                        ? t(`view.feed.filters.${String(row.original.type)}`)
                        : '';
                    return (
                        <FeedTypeIndicator
                            label={typeLabel}
                            type={row.original.type}
                        />
                    );
                }
            },
            {
                id: 'displayName',
                accessorFn: (row: FeedRow) => {
                    const userId = resolveFeedUserId(row);
                    return resolveFeedUserDisplayName(
                        row,
                        knownUsersById[userId],
                        friendLogNamesById[userId]
                    );
                },
                meta: { label: t('table.feed.user') },
                header: ({ column }: { column: Column<FeedRow, unknown> }) => (
                    <SortButton column={column} label={t('table.feed.user')} />
                ),
                cell: ({
                    row,
                    table
                }: {
                    row: Row<FeedRow>;
                    table: FeedTableInstance;
                }) => (
                    <FeedUserLink
                        actions={actions}
                        cachedDisplayName={
                            friendLogNamesById[resolveFeedUserId(row.original)]
                        }
                        className={resolveDisplayNameCellClassName(row, table)}
                        row={row.original}
                    />
                )
            },
            {
                id: 'detail',
                accessorFn: (row: FeedRow) =>
                    [
                        row?.location,
                        row?.worldName,
                        row?.statusDescription,
                        row?.avatarName,
                        row?.bio,
                        row?.message
                    ]
                        .filter(Boolean)
                        .join(' '),
                enableSorting: false,
                meta: { label: t('table.feed.detail') },
                header: () => t('table.feed.detail'),
                minSize: 100,
                cell: ({ row }: { row: Row<FeedRow> }) => (
                    <FeedDetailCell
                        loadingHistoryKey={loadingPreviousInstancesKey}
                        onNewInstance={actions.openFeedNewInstance}
                        onOpenPreviousInstances={onOpenPreviousInstances}
                        row={row.original}
                    />
                )
            }
        ],
        [
            actions,
            friendLogNamesById,
            knownUsersById,
            loadingPreviousInstancesKey,
            onOpenPreviousInstances,
            t
        ]
    );
}
