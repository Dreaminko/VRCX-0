import { useTranslation } from 'react-i18next';

import type { AppTable } from '@/components/data-table/appTable';
import { TableColumnVisibilityMenu } from '@/components/data-table/TableColumnVisibilityMenu';
import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarRefreshButton,
    ToolbarSearch,
    ToolbarStatus,
    ToolbarViews
} from '@/components/layout/ToolbarControls';

import {
    normalizeModerationSelectedTypes,
    resolveModerationTypeLabel
} from '../moderationPageState';
import type {
    ModerationLoadStatus,
    ModerationRow
} from '../moderationPageTypes';
import { ModerationTypeFilterDropdown } from './ModerationViewParts';

type ModerationPageToolbarProps = {
    selectedTypes: string[];
    onSelectedTypesChange: (value: string[]) => void;
    searchQuery: string;
    onSearchQueryChange: (value: string) => void;
    detail: string;
    currentUserId: string | null;
    loadStatus: ModerationLoadStatus;
    onRefresh: () => void;
    table: AppTable<ModerationRow>;
};

export function ModerationPageToolbar({
    selectedTypes,
    onSelectedTypesChange,
    searchQuery,
    onSearchQueryChange,
    detail,
    currentUserId,
    loadStatus,
    onRefresh,
    table
}: ModerationPageToolbarProps) {
    const { t } = useTranslation();
    const getModerationTypeLabel = (type: unknown) =>
        resolveModerationTypeLabel(type, t);

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    <ModerationTypeFilterDropdown
                        value={selectedTypes}
                        onChange={onSelectedTypesChange}
                        getTypeLabel={getModerationTypeLabel}
                        sanitizeTypes={normalizeModerationSelectedTypes}
                    />
                </ToolbarViews>

                <ToolbarSearch
                    value={searchQuery}
                    onValueChange={onSearchQueryChange}
                />

                <ToolbarActions>
                    <ToolbarRefreshButton
                        onRefresh={onRefresh}
                        loading={loadStatus === 'running'}
                        disabled={!currentUserId}
                    />
                    <TableColumnVisibilityMenu table={table} />
                </ToolbarActions>
            </PageToolbarRow>

            {detail ? <ToolbarStatus>{detail}</ToolbarStatus> : null}
        </PageToolbar>
    );
}
