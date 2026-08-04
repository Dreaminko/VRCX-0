import type { ComponentProps } from 'react';
import { useTranslation } from 'react-i18next';

import { DataTableSortButton } from '@/components/data-table/DataTableSortButton';
import { EmptyState } from '@/components/layout/PageScaffold';
import { ToolbarFilterMenu } from '@/components/layout/ToolbarControls';
import {
    DropdownMenuCheckboxItem,
    DropdownMenuGroup
} from '@/ui/shadcn/dropdown-menu';

import { FRIEND_LIST_SEARCH_FILTERS as SEARCH_FILTERS } from '../friendListState';

export { DataTableSortButton as SortButton };

export function FriendListEmptyState({
    title,
    description
}: ComponentProps<typeof EmptyState>) {
    return <EmptyState title={title} description={description} />;
}

export function FriendListSearchFilterDropdown({
    value,
    onChange
}: {
    value: Set<string>;
    onChange: (value: Set<string>) => void;
}) {
    const { t } = useTranslation();
    const activeFilters = value;

    return (
        <ToolbarFilterMenu activeCount={activeFilters.size}>
            <DropdownMenuGroup>
                {SEARCH_FILTERS.map((filter) => (
                    <DropdownMenuCheckboxItem
                        key={filter.id}
                        checked={activeFilters.has(filter.id)}
                        onClick={(event) => event.preventDefault()}
                        onCheckedChange={(checked) => {
                            const next = new Set(activeFilters);
                            if (checked) {
                                next.add(filter.id);
                            } else {
                                next.delete(filter.id);
                            }
                            onChange(next);
                        }}
                    >
                        {t(filter.labelKey)}
                    </DropdownMenuCheckboxItem>
                ))}
            </DropdownMenuGroup>
        </ToolbarFilterMenu>
    );
}
