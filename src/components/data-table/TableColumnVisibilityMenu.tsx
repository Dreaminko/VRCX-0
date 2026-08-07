import type { RowData } from '@tanstack/react-table';
import type { TFunction } from 'i18next';
import type { ReactElement, ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { ToolbarViewMenu } from '@/components/layout/ToolbarControls';
import {
    ContextMenu,
    ContextMenuCheckboxItem,
    ContextMenuContent,
    ContextMenuGroup,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuTrigger
} from '@/ui/shadcn/context-menu';
import {
    DropdownMenuCheckboxItem,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator
} from '@/ui/shadcn/dropdown-menu';

import type { AppTable } from './appTable';
import {
    getColumnOrderLocked,
    getToggleableColumns,
    hasColumnOrderLock,
    resetTableLayout,
    resolveColumnLabel,
    setColumnOrderLocked
} from './tableColumnLayout';

type ResetTableLayoutHandler<TData extends RowData> = (
    table: AppTable<TData>
) => void;

function renderColumnLockLabel(locked: boolean, t: TFunction) {
    return locked
        ? t('table.label.unlock_column_order')
        : t('table.label.lock_column_order');
}

export function TableColumnVisibilityMenu<TData extends RowData>({
    table,
    onResetLayout
}: {
    table: AppTable<TData>;
    onResetLayout?: ResetTableLayoutHandler<TData>;
}) {
    const { t } = useTranslation();

    const columns = getToggleableColumns(table.getAllLeafColumns());
    const showColumnOrderLock = hasColumnOrderLock(table);

    if (!columns.length && !showColumnOrderLock) {
        return null;
    }

    const columnOrderLocked = getColumnOrderLocked(table);

    return (
        <ToolbarViewMenu>
            <DropdownMenuGroup>
                <DropdownMenuLabel>
                    {t('table.label.table_layout')}
                </DropdownMenuLabel>
                <DropdownMenuItem
                    closeOnClick={false}
                    onClick={(event) => {
                        event.preventDefault();
                        resetTableLayout(table, onResetLayout);
                    }}
                >
                    {t('table.action.reset_columns')}
                </DropdownMenuItem>
                {showColumnOrderLock ? (
                    <DropdownMenuItem
                        closeOnClick={false}
                        onClick={(event) => {
                            event.preventDefault();
                            setColumnOrderLocked(table, !columnOrderLocked);
                        }}
                    >
                        {renderColumnLockLabel(columnOrderLocked, t)}
                    </DropdownMenuItem>
                ) : null}
            </DropdownMenuGroup>
            {columns.length ? (
                <>
                    <DropdownMenuSeparator />
                    <DropdownMenuGroup>
                        <DropdownMenuLabel>
                            {t('table.label.columns')}
                        </DropdownMenuLabel>
                        {columns.map((column) => (
                            <DropdownMenuCheckboxItem
                                key={column.id}
                                checked={column.getIsVisible()}
                                onCheckedChange={(checked) =>
                                    column.toggleVisibility(checked === true)
                                }
                                onClick={(event) => event.preventDefault()}
                            >
                                <span className="min-w-0 flex-1 truncate">
                                    {resolveColumnLabel(column)}
                                </span>
                            </DropdownMenuCheckboxItem>
                        ))}
                    </DropdownMenuGroup>
                </>
            ) : null}
        </ToolbarViewMenu>
    );
}

export function TableColumnHeaderContextMenu<TData extends RowData>({
    table,
    onResetLayout,
    children,
    className = 'w-56'
}: {
    table: AppTable<TData>;
    onResetLayout?: ResetTableLayoutHandler<TData>;
    children: ReactNode;
    className?: string;
}) {
    const { t } = useTranslation();

    const allLeafColumns = table?.getAllLeafColumns?.() ?? [];
    const columns = getToggleableColumns(allLeafColumns);
    const columnOrderLocked = getColumnOrderLocked(table);
    const showColumnOrderLock = hasColumnOrderLock(table);
    const showReset = Boolean(
        onResetLayout ||
        table?.resetColumnVisibility ||
        table?.setColumnOrder ||
        table?.setColumnSizing
    );
    const showMenu = Boolean(
        columns.length || showColumnOrderLock || showReset
    );

    if (!showMenu) {
        return children;
    }

    return (
        <ContextMenu>
            <ContextMenuTrigger render={children as ReactElement} />
            <ContextMenuContent className={className}>
                {columns.length ? (
                    <ContextMenuGroup>
                        {columns.map((column) => (
                            <ContextMenuCheckboxItem
                                key={column.id}
                                checked={column.getIsVisible()}
                                onCheckedChange={(checked) =>
                                    column.toggleVisibility(checked === true)
                                }
                                onClick={(event) => event.preventDefault()}
                            >
                                <span className="min-w-0 flex-1 truncate">
                                    {resolveColumnLabel(column)}
                                </span>
                            </ContextMenuCheckboxItem>
                        ))}
                    </ContextMenuGroup>
                ) : null}
                {columns.length && (showColumnOrderLock || showReset) ? (
                    <ContextMenuSeparator />
                ) : null}
                {showColumnOrderLock || showReset ? (
                    <ContextMenuGroup>
                        {showColumnOrderLock ? (
                            <ContextMenuCheckboxItem
                                checked={columnOrderLocked}
                                onCheckedChange={(checked) =>
                                    setColumnOrderLocked(
                                        table,
                                        checked === true
                                    )
                                }
                                onClick={(event) => event.preventDefault()}
                            >
                                {renderColumnLockLabel(columnOrderLocked, t)}
                            </ContextMenuCheckboxItem>
                        ) : null}
                        {showReset ? (
                            <ContextMenuItem
                                inset={showColumnOrderLock}
                                onClick={() =>
                                    resetTableLayout(table, onResetLayout)
                                }
                            >
                                {t('table.action.reset_columns')}
                            </ContextMenuItem>
                        ) : null}
                    </ContextMenuGroup>
                ) : null}
            </ContextMenuContent>
        </ContextMenu>
    );
}
