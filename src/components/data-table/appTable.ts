import type {
    Cell,
    CellContext,
    Column,
    ColumnDef,
    Header,
    ReactTable,
    Row,
    RowData,
    TableOptions
} from '@tanstack/react-table';
import {
    columnOrderingFeature,
    columnResizingFeature,
    columnSizingFeature,
    columnVisibilityFeature,
    createExpandedRowModel,
    createPaginatedRowModel,
    createSortedRowModel,
    rowExpandingFeature,
    rowPaginationFeature,
    rowSortingFeature,
    tableFeatures,
    useTable
} from '@tanstack/react-table';

const appTableFeatures = tableFeatures({
    rowSortingFeature,
    rowPaginationFeature,
    rowExpandingFeature,
    columnOrderingFeature,
    columnVisibilityFeature,
    columnSizingFeature,
    columnResizingFeature,
    sortedRowModel: createSortedRowModel(),
    paginatedRowModel: createPaginatedRowModel(),
    expandedRowModel: createExpandedRowModel()
});

type AppTableFeatures = typeof appTableFeatures;

export type AppColumnDef<TData extends RowData, TValue = unknown> = ColumnDef<
    AppTableFeatures,
    TData,
    TValue
>;
export type AppColumn<TData extends RowData, TValue = unknown> = Column<
    AppTableFeatures,
    TData,
    TValue
>;
export type AppRow<TData extends RowData> = Row<AppTableFeatures, TData>;
export type AppTable<TData extends RowData> = ReactTable<
    AppTableFeatures,
    TData
>;
export type AppCell<TData extends RowData, TValue = unknown> = Cell<
    AppTableFeatures,
    TData,
    TValue
>;
export type AppHeader<TData extends RowData, TValue = unknown> = Header<
    AppTableFeatures,
    TData,
    TValue
>;
export type AppCellContext<
    TData extends RowData,
    TValue = unknown
> = CellContext<AppTableFeatures, TData, TValue>;

export function useAppTable<TData extends RowData>(
    options: Omit<TableOptions<AppTableFeatures, TData>, 'features'>
): AppTable<TData> {
    return useTable({ ...options, features: appTableFeatures });
}
