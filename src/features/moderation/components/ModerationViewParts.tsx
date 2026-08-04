import { DataTableSortButton } from '@/components/data-table/DataTableSortButton';
import { EmptyState } from '@/components/layout/PageScaffold';
import { ToolbarFilterMenu } from '@/components/layout/ToolbarControls';
import { moderationTypes } from '@/shared/constants/moderation';
import {
    DropdownMenuCheckboxItem,
    DropdownMenuGroup
} from '@/ui/shadcn/dropdown-menu';

export { DataTableSortButton as SortButton };

type ModerationEmptyStateProps = {
    title?: string;
    description?: string;
};

type ModerationTypeFilterDropdownProps = {
    value?: string[];
    onChange: (value: string[]) => void;
    getTypeLabel: (type: string) => string;
    sanitizeTypes?: (types: string[]) => string[];
};

export function ModerationEmptyState({
    title,
    description
}: ModerationEmptyStateProps) {
    return <EmptyState title={title} description={description} />;
}

export function ModerationTypeFilterDropdown({
    value,
    onChange,
    getTypeLabel,
    sanitizeTypes = (types) => types
}: ModerationTypeFilterDropdownProps) {
    const selectedTypes = Array.isArray(value) ? value : [];

    return (
        <ToolbarFilterMenu activeCount={selectedTypes.length}>
            <DropdownMenuGroup>
                {moderationTypes.map((type) => (
                    <DropdownMenuCheckboxItem
                        key={type}
                        checked={selectedTypes.includes(type)}
                        onCheckedChange={(checked) => {
                            const next = checked
                                ? [...selectedTypes, type]
                                : selectedTypes.filter(
                                      (entry) => entry !== type
                                  );
                            onChange(sanitizeTypes(next));
                        }}
                        onClick={(event) => event.preventDefault()}
                    >
                        {getTypeLabel(type)}
                    </DropdownMenuCheckboxItem>
                ))}
            </DropdownMenuGroup>
        </ToolbarFilterMenu>
    );
}
