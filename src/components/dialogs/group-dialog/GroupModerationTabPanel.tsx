import { DownloadIcon, Loader2Icon, RefreshCwIcon } from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import type {
    EntityRecord,
    GroupProfileRecord
} from '@/domain/entities/profileEntities';
import { formatDateFilter } from '@/lib/dateTime';
import { Button } from '@/ui/shadcn/button';
import { Checkbox } from '@/ui/shadcn/checkbox';
import { Input } from '@/ui/shadcn/input';
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow
} from '@/ui/shadcn/table';
import { TabsContent } from '@/ui/shadcn/tabs';

import { downloadJsonFile } from './groupDialogDownloads';
import { GroupListState } from './GroupListState';
import {
    getGroupModerationActions,
    moderationRowDate,
    moderationRowLabel,
    moderationRowNote,
    moderationRowRoles,
    moderationRowSearchText,
    moderationRowStatus,
    moderationRowUserId,
    type GroupModerationAction,
    type GroupModerationTab
} from './groupModerationRows';
import { ModerationStatusBadge } from './ModerationStatusBadge';

function text(value: unknown): string {
    return typeof value === 'string' ? value : '';
}

const ALL_ROLES_VALUE = 'all';

export interface GroupModerationServerSelectOption {
    label: string;
    value: string;
}

export interface GroupModerationServerControl {
    query: string;
    onQueryChange: (value: string) => void;
    sort: string;
    onSortChange: (value: string) => void;
    sortOptions: GroupModerationServerSelectOption[];
    roleId: string;
    onRoleChange: (value: string) => void;
    roleOptions: GroupModerationServerSelectOption[];
    hasMore: boolean;
    loadingMore: boolean;
    onLoadMore: () => void;
    loadedCount: number;
}

export function GroupModerationTabPanel({
    actionKey,
    activeTab,
    error,
    focusedUserId,
    group,
    loading,
    onFocusRow,
    onPageIndexChange,
    onPageSizeChange,
    onReload,
    onRunAction,
    onSearchChange,
    onToggleAllVisible,
    onToggleRow,
    pageIndex,
    pageSize,
    rows,
    search,
    selectable = false,
    selectedIds,
    server,
    tab,
    toolbarExtra
}: {
    actionKey: string;
    activeTab: string;
    error: string;
    focusedUserId?: string;
    group: GroupProfileRecord;
    loading: boolean;
    onFocusRow?: (userId: string) => void;
    onPageIndexChange: (value: number) => void;
    onPageSizeChange: (value: number) => void;
    onReload: () => void;
    onRunAction: (action: GroupModerationAction, row: EntityRecord) => void;
    onSearchChange: (value: string) => void;
    onToggleAllVisible?: (userIds: string[], checked: boolean) => void;
    onToggleRow?: (userId: string, checked: boolean) => void;
    pageIndex: number;
    pageSize: number;
    rows: EntityRecord[];
    search: string;
    selectable?: boolean;
    selectedIds?: ReadonlySet<string>;
    server?: GroupModerationServerControl;
    tab: GroupModerationTab;
    toolbarExtra?: ReactNode;
}) {
    const { t } = useTranslation();
    const filteredRows = server
        ? rows
        : rows.filter((row) => {
              const query = search.trim().toLowerCase();
              return (
                  !query || moderationRowSearchText(row, group).includes(query)
              );
          });
    const totalPages = Math.max(1, Math.ceil(filteredRows.length / pageSize));
    const currentPageIndex = Math.min(pageIndex, totalPages - 1);
    const visibleRows = server
        ? rows
        : filteredRows.slice(
              currentPageIndex * pageSize,
              currentPageIndex * pageSize + pageSize
          );
    const visibleUserIds = visibleRows
        .map((row) => moderationRowUserId(row))
        .filter(Boolean);
    const allVisibleSelected =
        selectable &&
        visibleUserIds.length > 0 &&
        visibleUserIds.every((userId) => selectedIds?.has(userId));
    const columnCount = selectable ? 6 : 5;
    const serverQueryActive = Boolean(server && server.query.trim());
    const clientQueryActive = !server && Boolean(search.trim());
    const emptyMessage = server
        ? serverQueryActive
            ? t('common.no_matching_records')
            : t('dialog.group.empty.no_rows')
        : clientQueryActive && rows.length
          ? t('common.no_matching_records')
          : t('dialog.group.empty.no_rows');

    return (
        <TabsContent
            value={tab.value}
            className="m-0 max-h-[65vh] overflow-auto pt-4"
        >
            <div className="mb-3 flex items-center justify-between gap-3">
                <div className="flex items-center gap-2">
                    <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        disabled={loading}
                        onClick={onReload}
                    >
                        <RefreshCwIcon data-icon="inline-start" />
                        {t('common.actions.refresh')}
                    </Button>
                    <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        disabled={!rows.length}
                        onClick={() =>
                            downloadJsonFile(
                                `${group.id}_${activeTab}.json`,
                                rows
                            )
                        }
                    >
                        <DownloadIcon data-icon="inline-start" />
                        JSON
                    </Button>
                    {toolbarExtra}
                    <span className="text-muted-foreground text-sm">
                        {server
                            ? server.loadedCount
                            : `${filteredRows.length}/${rows.length}`}
                    </span>
                </div>
                <div className="flex items-center gap-2">
                    <Input
                        value={server ? server.query : search}
                        onChange={(event) =>
                            server
                                ? server.onQueryChange(event.target.value)
                                : onSearchChange(event.target.value)
                        }
                        placeholder={t('dialog.group.dynamic.search_value', {
                            value: tab.label.toLowerCase()
                        })}
                        className="h-8 w-64"
                    />
                    {server ? (
                        <>
                            <Select
                                value={server.sort}
                                disabled={serverQueryActive}
                                onValueChange={(value) =>
                                    value && server.onSortChange(value)
                                }
                            >
                                <SelectTrigger size="sm" className="w-44">
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectGroup>
                                        {server.sortOptions.map((option) => (
                                            <SelectItem
                                                key={option.value}
                                                value={option.value}
                                            >
                                                {option.label}
                                            </SelectItem>
                                        ))}
                                    </SelectGroup>
                                </SelectContent>
                            </Select>
                            {server.roleOptions.length ? (
                                <Select
                                    value={server.roleId || ALL_ROLES_VALUE}
                                    disabled={serverQueryActive}
                                    onValueChange={(value) =>
                                        server.onRoleChange(
                                            value === ALL_ROLES_VALUE
                                                ? ''
                                                : (value ?? '')
                                        )
                                    }
                                >
                                    <SelectTrigger size="sm" className="w-40">
                                        <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectGroup>
                                            {server.roleOptions.map(
                                                (option) => (
                                                    <SelectItem
                                                        key={
                                                            option.value ||
                                                            ALL_ROLES_VALUE
                                                        }
                                                        value={
                                                            option.value ||
                                                            ALL_ROLES_VALUE
                                                        }
                                                    >
                                                        {option.label}
                                                    </SelectItem>
                                                )
                                            )}
                                        </SelectGroup>
                                    </SelectContent>
                                </Select>
                            ) : null}
                        </>
                    ) : (
                        <Select
                            value={String(pageSize)}
                            onValueChange={(value) =>
                                onPageSizeChange(
                                    Number.parseInt(value ?? '', 10) || 25
                                )
                            }
                        >
                            <SelectTrigger size="sm" className="w-24">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectGroup>
                                    {[10, 25, 50, 100].map((size) => (
                                        <SelectItem
                                            key={size}
                                            value={String(size)}
                                        >
                                            {size}
                                        </SelectItem>
                                    ))}
                                </SelectGroup>
                            </SelectContent>
                        </Select>
                    )}
                </div>
            </div>
            {loading ? (
                <GroupListState
                    title={t('dialog.group.dynamic.no_value', {
                        value: tab.label.toLowerCase()
                    })}
                    loading
                />
            ) : null}
            {error ? (
                <GroupListState
                    title={t('dialog.group.dynamic.no_value', {
                        value: tab.label.toLowerCase()
                    })}
                    error={error}
                />
            ) : null}
            {!loading && !error ? (
                <div className="overflow-auto rounded-md border">
                    <Table>
                        <TableHeader className="vrcx-0-table-header sticky top-0">
                            <TableRow>
                                {selectable ? (
                                    <TableHead className="w-10">
                                        <Checkbox
                                            checked={allVisibleSelected}
                                            disabled={!visibleUserIds.length}
                                            aria-label={t(
                                                'dialog.group_member_moderation.select_all'
                                            )}
                                            onCheckedChange={(checked) =>
                                                onToggleAllVisible?.(
                                                    visibleUserIds,
                                                    Boolean(checked)
                                                )
                                            }
                                        />
                                    </TableHead>
                                ) : null}
                                <TableHead className="w-56">
                                    {t('dialog.group.label.user')}
                                </TableHead>
                                <TableHead>
                                    {t('dialog.group_member_moderation.roles')}{' '}
                                    /{' '}
                                    {t(
                                        'dialog.group_member_moderation.description'
                                    )}
                                </TableHead>
                                <TableHead className="w-44">
                                    {t('dialog.group.label.status')}
                                </TableHead>
                                <TableHead className="w-44">
                                    {t('dialog.group.label.date')}
                                </TableHead>
                                <TableHead className="w-48 text-right">
                                    {t('dialog.group.label.actions')}
                                </TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {visibleRows.length ? (
                                visibleRows.map((row, index) => {
                                    const userId = moderationRowUserId(row);
                                    const label = moderationRowLabel(row);
                                    const date = moderationRowDate(row);
                                    const actions = getGroupModerationActions(
                                        tab.value,
                                        row,
                                        t
                                    );
                                    const isFocused = Boolean(
                                        userId && userId === focusedUserId
                                    );
                                    return (
                                        <TableRow
                                            key={`${label}:${date}:${index}`}
                                            className={
                                                isFocused
                                                    ? 'bg-muted/50'
                                                    : undefined
                                            }
                                        >
                                            {selectable ? (
                                                <TableCell className="align-top">
                                                    <Checkbox
                                                        checked={Boolean(
                                                            userId &&
                                                            selectedIds?.has(
                                                                userId
                                                            )
                                                        )}
                                                        disabled={!userId}
                                                        aria-label={label}
                                                        onCheckedChange={(
                                                            checked
                                                        ) =>
                                                            userId &&
                                                            onToggleRow?.(
                                                                userId,
                                                                Boolean(checked)
                                                            )
                                                        }
                                                    />
                                                </TableCell>
                                            ) : null}
                                            <TableCell className="align-top">
                                                {userId ? (
                                                    <Button
                                                        type="button"
                                                        variant="ghost"
                                                        className="hover:text-primary h-auto max-w-52 justify-start truncate p-0 text-left font-medium"
                                                        onClick={() =>
                                                            onFocusRow?.(
                                                                userId
                                                            )
                                                        }
                                                    >
                                                        {label}
                                                    </Button>
                                                ) : (
                                                    <span className="font-medium">
                                                        {label}
                                                    </span>
                                                )}
                                                <div className="text-muted-foreground truncate font-mono text-xs">
                                                    {userId ||
                                                        text(row.id) ||
                                                        '—'}
                                                </div>
                                            </TableCell>
                                            <TableCell className="text-muted-foreground align-top text-xs whitespace-normal">
                                                {moderationRowRoles(
                                                    row,
                                                    group
                                                ) ||
                                                    moderationRowNote(row) ||
                                                    '—'}
                                            </TableCell>
                                            <TableCell className="align-top text-xs whitespace-normal">
                                                <ModerationStatusBadge
                                                    status={moderationRowStatus(
                                                        row
                                                    )}
                                                />
                                            </TableCell>
                                            <TableCell className="text-muted-foreground align-top text-xs">
                                                {date
                                                    ? formatDateFilter(
                                                          date,
                                                          'long'
                                                      )
                                                    : '—'}
                                            </TableCell>
                                            <TableCell className="align-top">
                                                <div className="flex justify-end gap-2">
                                                    {actions.map((action) => {
                                                        const nextActionKey = `${activeTab}:${action.key}:${userId}`;
                                                        return (
                                                            <Button
                                                                key={action.key}
                                                                type="button"
                                                                size="sm"
                                                                variant={
                                                                    action.destructive
                                                                        ? 'outline'
                                                                        : 'secondary'
                                                                }
                                                                disabled={Boolean(
                                                                    actionKey
                                                                )}
                                                                onClick={() => {
                                                                    onRunAction(
                                                                        action,
                                                                        row
                                                                    );
                                                                }}
                                                            >
                                                                {actionKey ===
                                                                nextActionKey
                                                                    ? '...'
                                                                    : action.label}
                                                            </Button>
                                                        );
                                                    })}
                                                </div>
                                            </TableCell>
                                        </TableRow>
                                    );
                                })
                            ) : (
                                <TableRow>
                                    <TableCell
                                        colSpan={columnCount}
                                        className="text-muted-foreground py-8 text-center text-sm"
                                    >
                                        {emptyMessage}
                                    </TableCell>
                                </TableRow>
                            )}
                        </TableBody>
                    </Table>
                </div>
            ) : null}
            {!loading && !error && server ? (
                <div className="mt-3 flex items-center justify-between">
                    <span className="text-muted-foreground text-sm">
                        {t('dialog.group_member_moderation.loaded_count', {
                            count: server.loadedCount
                        })}
                    </span>
                    <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        disabled={!server.hasMore || server.loadingMore}
                        onClick={server.onLoadMore}
                    >
                        {server.loadingMore ? (
                            <Loader2Icon
                                data-icon="inline-start"
                                className="animate-spin"
                            />
                        ) : null}
                        {t('common.load_more')}
                    </Button>
                </div>
            ) : null}
            {!loading && !error && !server ? (
                <div className="mt-3 flex items-center justify-between">
                    <span className="text-muted-foreground text-sm">
                        {t('dialog.group.label.page')} {currentPageIndex + 1} /{' '}
                        {totalPages}
                    </span>
                    <div className="flex gap-2">
                        <Button
                            type="button"
                            size="sm"
                            variant="outline"
                            disabled={currentPageIndex <= 0}
                            onClick={() =>
                                onPageIndexChange(
                                    Math.max(0, currentPageIndex - 1)
                                )
                            }
                        >
                            {t('table.pagination.previous')}
                        </Button>
                        <Button
                            type="button"
                            size="sm"
                            variant="outline"
                            disabled={currentPageIndex >= totalPages - 1}
                            onClick={() =>
                                onPageIndexChange(
                                    Math.min(
                                        totalPages - 1,
                                        currentPageIndex + 1
                                    )
                                )
                            }
                        >
                            {t('table.pagination.next')}
                        </Button>
                    </div>
                </div>
            ) : null}
        </TabsContent>
    );
}
