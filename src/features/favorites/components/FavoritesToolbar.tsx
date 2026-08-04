import {
    ArrowUpDownIcon,
    DownloadIcon,
    ExternalLinkIcon,
    UploadIcon
} from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarOverflowMenu,
    ToolbarRefreshButton,
    ToolbarSearch,
    ToolbarViewMenu,
    ToolbarViews
} from '@/components/layout/ToolbarControls';
import {
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuSeparator
} from '@/ui/shadcn/dropdown-menu';
import { Field, FieldContent, FieldGroup, FieldLabel } from '@/ui/shadcn/field';
import { InputGroupButton } from '@/ui/shadcn/input-group';
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';
import { ToggleGroup, ToggleGroupItem } from '@/ui/shadcn/toggle-group';

import {
    FAVORITES_DENSITY_OPTIONS,
    type FavoritesDensity
} from '../favoritesDensity';
import type { FavoriteKind } from '../favoritesTypes';

type FavoritesToolbarProps = {
    kind: FavoriteKind;
    sortValue: string;
    searchQuery: string;
    searchPlaceholder: string;
    searchMode: string;
    density: FavoritesDensity;
    refreshing: boolean;
    onSortValueChange: (value: string) => void;
    onSearchChange: (value: string) => void;
    onSearchModeChange: (mode: string) => void;
    onDensityChange: (value: FavoritesDensity) => void;
    onRefresh: () => void;
    onImport: () => void;
    onExport: () => void;
    onManageShares?: () => void;
};

function FavoritesToolbar({
    kind,
    sortValue,
    searchQuery,
    searchPlaceholder,
    searchMode,
    density,
    refreshing,
    onSortValueChange,
    onSearchChange,
    onSearchModeChange,
    onDensityChange,
    onRefresh,
    onImport,
    onExport,
    onManageShares
}: FavoritesToolbarProps) {
    const { t } = useTranslation();
    const sortItems = [
        { value: 'name', label: t('view.search.avatar.sort_name') },
        { value: 'date', label: t('view.favorite.label.sort_by_date') },
        ...(kind === 'world'
            ? [
                  {
                      value: 'players',
                      label: t('view.favorite.label.sort_by_players')
                  }
              ]
            : [])
    ];

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    <Select
                        value={sortValue}
                        items={sortItems}
                        onValueChange={(value) =>
                            onSortValueChange(value ?? '')
                        }
                    >
                        <SelectTrigger className="max-w-56 min-w-40 shrink-0">
                            <span className="flex min-w-0 items-center gap-2">
                                <ArrowUpDownIcon className="text-muted-foreground size-4 shrink-0" />
                                <SelectValue
                                    placeholder={t(
                                        'view.favorite.label.sort_favorites'
                                    )}
                                />
                            </span>
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
                </ToolbarViews>

                <ToolbarSearch
                    value={searchQuery}
                    onValueChange={onSearchChange}
                    placeholder={searchPlaceholder}
                    className={kind === 'world' ? 'sm:w-88' : undefined}
                    trailing={
                        kind === 'world' ? (
                            <>
                                <InputGroupButton
                                    type="button"
                                    variant={
                                        searchMode === 'name'
                                            ? 'secondary'
                                            : 'ghost'
                                    }
                                    onClick={() => onSearchModeChange('name')}
                                >
                                    {t('view.favorite.worlds.search_mode_name')}
                                </InputGroupButton>
                                <InputGroupButton
                                    type="button"
                                    variant={
                                        searchMode === 'tag'
                                            ? 'secondary'
                                            : 'ghost'
                                    }
                                    onClick={() => onSearchModeChange('tag')}
                                >
                                    {t('view.favorite.worlds.search_mode_tag')}
                                </InputGroupButton>
                            </>
                        ) : null
                    }
                />

                <ToolbarActions>
                    <ToolbarRefreshButton
                        onRefresh={onRefresh}
                        loading={refreshing}
                    />
                    <ToolbarViewMenu contentClassName="p-3">
                        <FieldGroup
                            onClick={(event) => event.stopPropagation()}
                        >
                            <Field>
                                <FieldContent>
                                    <FieldLabel>
                                        {t('view.friends_locations.density')}
                                    </FieldLabel>
                                </FieldContent>
                                <ToggleGroup
                                    variant="outline"
                                    size="sm"
                                    spacing={1}
                                    value={density ? [density] : []}
                                    onValueChange={(nextValue) => {
                                        if (nextValue[0]) {
                                            onDensityChange(
                                                nextValue[0] as FavoritesDensity
                                            );
                                        }
                                    }}
                                    className="grid w-full grid-cols-2"
                                >
                                    {FAVORITES_DENSITY_OPTIONS.map((option) => (
                                        <ToggleGroupItem
                                            key={option.value}
                                            value={option.value}
                                            aria-label={t(option.labelKey)}
                                            className="w-full min-w-0 justify-center px-2"
                                        >
                                            <span className="truncate">
                                                {t(option.labelKey)}
                                            </span>
                                        </ToggleGroupItem>
                                    ))}
                                </ToggleGroup>
                            </Field>
                        </FieldGroup>
                    </ToolbarViewMenu>
                    <ToolbarOverflowMenu>
                        <DropdownMenuGroup>
                            <DropdownMenuItem onClick={onImport}>
                                <UploadIcon data-icon="inline-start" />
                                {t('view.favorite.import')}
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={onExport}>
                                <DownloadIcon data-icon="inline-start" />
                                {t('view.favorite.export')}
                            </DropdownMenuItem>
                        </DropdownMenuGroup>
                        {kind === 'world' && onManageShares ? (
                            <>
                                <DropdownMenuSeparator />
                                <DropdownMenuGroup>
                                    <DropdownMenuItem onClick={onManageShares}>
                                        <ExternalLinkIcon data-icon="inline-start" />
                                        {t(
                                            'view.favorite.share_collection.action.open_manage'
                                        )}
                                    </DropdownMenuItem>
                                </DropdownMenuGroup>
                            </>
                        ) : null}
                    </ToolbarOverflowMenu>
                </ToolbarActions>
            </PageToolbarRow>
        </PageToolbar>
    );
}

export { FavoritesToolbar };
