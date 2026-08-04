import type { Table as ReactTable } from '@tanstack/react-table';
import { LayoutGridIcon, ListIcon } from 'lucide-react';
import type { Dispatch, SetStateAction } from 'react';
import { useTranslation } from 'react-i18next';

import { TableColumnVisibilityMenu } from '@/components/data-table/TableColumnVisibilityMenu';
import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarRefreshButton,
    ToolbarSearch,
    ToolbarSegmented,
    ToolbarViews,
    type ToolbarSegmentOption
} from '@/components/layout/ToolbarControls';
import { useRuntimeStore } from '@/state/runtimeStore';

import type {
    MyAvatarsGridDensity,
    MyAvatarsLoadStatus,
    MyAvatarRow,
    MyAvatarsViewMode
} from '../myAvatarsTypes';
import { GridSettingsMenu, MyAvatarFilterPopover } from './MyAvatarsViewParts';

type MyAvatarsToolbarProps = {
    viewMode: MyAvatarsViewMode;
    activeFilterCount: number;
    allTags: string[];
    releaseStatusFilter: string;
    platformFilter: string;
    tagFilters: Set<string>;
    loadStatus: MyAvatarsLoadStatus;
    searchQuery: string;
    gridDensity: MyAvatarsGridDensity;
    table: ReactTable<MyAvatarRow>;
    onViewModeChange: (value: MyAvatarsViewMode) => void;
    onReleaseStatusChange: (value: string) => void;
    onPlatformChange: (value: string) => void;
    onTagFiltersChange: Dispatch<SetStateAction<Set<string>>>;
    onClearFilters: () => void;
    onSearchChange: (value: string) => void;
    onGridDensityChange: (value: string) => void;
    onRefresh: () => void;
};

export function MyAvatarsToolbar({
    viewMode,
    activeFilterCount,
    allTags,
    releaseStatusFilter,
    platformFilter,
    tagFilters,
    loadStatus,
    searchQuery,
    gridDensity,
    table,
    onViewModeChange,
    onReleaseStatusChange,
    onPlatformChange,
    onTagFiltersChange,
    onClearFilters,
    onSearchChange,
    onGridDensityChange,
    onRefresh
}: MyAvatarsToolbarProps) {
    const { t } = useTranslation();
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const viewModeOptions: ToolbarSegmentOption<MyAvatarsViewMode>[] = [
        {
            value: 'grid',
            label: t('view.my_avatars.action.show_avatar_grid'),
            icon: LayoutGridIcon
        },
        {
            value: 'table',
            label: t('view.my_avatars.action.show_avatar_table'),
            icon: ListIcon
        }
    ];

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    <ToolbarSegmented
                        iconOnly
                        value={viewMode}
                        onValueChange={onViewModeChange}
                        options={viewModeOptions}
                    />
                    <MyAvatarFilterPopover
                        activeFilterCount={activeFilterCount}
                        allTags={allTags}
                        releaseStatusFilter={releaseStatusFilter}
                        platformFilter={platformFilter}
                        tagFilters={tagFilters}
                        onReleaseStatusChange={onReleaseStatusChange}
                        onPlatformChange={onPlatformChange}
                        onTagFiltersChange={onTagFiltersChange}
                        onClearFilters={onClearFilters}
                    />
                </ToolbarViews>

                <ToolbarSearch
                    value={searchQuery}
                    onValueChange={onSearchChange}
                />

                <ToolbarActions>
                    <ToolbarRefreshButton
                        onRefresh={onRefresh}
                        loading={loadStatus === 'running'}
                        disabled={!currentUserId}
                        label={t(
                            'view.my_avatars.action.refresh_avatar_inventory'
                        )}
                    />
                    {viewMode === 'grid' ? (
                        <GridSettingsMenu
                            gridDensity={gridDensity}
                            onGridDensityChange={onGridDensityChange}
                        />
                    ) : null}
                    {viewMode === 'table' ? (
                        <TableColumnVisibilityMenu table={table} />
                    ) : null}
                </ToolbarActions>
            </PageToolbarRow>
        </PageToolbar>
    );
}
