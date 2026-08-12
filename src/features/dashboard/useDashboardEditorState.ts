import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import {
    generateDashboardRowId,
    type Dashboard,
    type DashboardDirection,
    type DashboardPanel,
    type DashboardRow
} from '@/repositories/dashboardRepository';

import { cloneDashboardRows } from './dashboardConfig';

type DashboardEditorStateProps = {
    consumeEditingDashboardId: (dashboardId: string) => boolean;
    dashboard: Dashboard | null;
    editingDashboardId: string | null;
    loaded: boolean;
    saveDashboard: (
        dashboardId: string,
        patch: Pick<Dashboard, 'name' | 'rows'>
    ) => Promise<unknown>;
};

export function useDashboardEditorState({
    consumeEditingDashboardId,
    dashboard,
    editingDashboardId,
    loaded,
    saveDashboard
}: DashboardEditorStateProps) {
    const { t } = useTranslation();
    const [isEditing, setIsEditing] = useState(false);
    const [editName, setEditName] = useState('');
    const [editRows, setEditRows] = useState<DashboardRow[]>([]);
    const [isSaving, setIsSaving] = useState(false);
    const previousDashboardIdRef = useRef<string | null>(null);

    const resetEditDraft = useCallback(() => {
        setEditName(dashboard?.name || '');
        setEditRows(cloneDashboardRows(dashboard?.rows));
    }, [dashboard]);

    useEffect(() => {
        if (!dashboard) {
            setIsEditing(false);
            setEditName('');
            setEditRows([]);
            return;
        }

        resetEditDraft();
    }, [dashboard, resetEditDraft]);

    useEffect(() => {
        if (!loaded || !dashboard?.id) {
            return;
        }

        if (previousDashboardIdRef.current !== dashboard.id) {
            previousDashboardIdRef.current = dashboard.id;
            if (editingDashboardId !== dashboard.id) {
                setIsEditing(false);
            }
        }

        if (
            editingDashboardId === dashboard.id &&
            consumeEditingDashboardId(dashboard.id)
        ) {
            setIsEditing(true);
        }
    }, [consumeEditingDashboardId, dashboard?.id, editingDashboardId, loaded]);

    const handleAddRow = (
        panelCount: number,
        direction: DashboardDirection = 'horizontal'
    ) => {
        setEditRows((current) => [
            ...current,
            {
                id: generateDashboardRowId(),
                direction,
                panels: Array.from(
                    { length: panelCount },
                    (): DashboardPanel | null => null
                )
            }
        ]);
    };

    const handleUpdatePanel = (
        rowIndex: number,
        panelIndex: number,
        nextPanel: DashboardPanel | null
    ) => {
        setEditRows((current) =>
            current.map((row, currentRowIndex) => {
                if (currentRowIndex !== rowIndex) {
                    return row;
                }

                const panels = row.panels.slice(0, 2);
                panels[panelIndex] = nextPanel;
                return {
                    ...row,
                    panels
                };
            })
        );
    };

    const handleRemovePanel = (rowIndex: number, panelIndex: number) => {
        setEditRows((current) =>
            current
                .map((row, currentRowIndex) => {
                    if (currentRowIndex !== rowIndex) {
                        return row;
                    }

                    const panels = row.panels.slice(0, 2);
                    panels.splice(panelIndex, 1);
                    return {
                        ...row,
                        panels
                    };
                })
                .filter((row) => row.panels.length > 0)
        );
    };

    const handleRemoveRow = (rowIndex: number) => {
        setEditRows((current) =>
            current.filter((_, index) => index !== rowIndex)
        );
    };

    const handleDirectionChange = (
        rowIndex: number,
        direction: DashboardDirection
    ) => {
        setEditRows((current) =>
            current.map((row, index) =>
                index === rowIndex
                    ? {
                          ...row,
                          direction:
                              direction === 'vertical'
                                  ? 'vertical'
                                  : 'horizontal'
                      }
                    : row
            )
        );
    };

    const handleSave = async () => {
        if (!dashboard) {
            return;
        }

        setIsSaving(true);
        try {
            await saveDashboard(dashboard.id, {
                name:
                    editName.trim() ||
                    dashboard.name ||
                    t('dashboard.default_name'),
                rows: editRows
            });
            setIsEditing(false);
            toast.success(t('view.dashboard.success.dashboard_saved'));
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t('view.dashboard.toast.failed_to_save_dashboard')
            );
        } finally {
            setIsSaving(false);
        }
    };

    function cancelEditing() {
        setIsEditing(false);
        resetEditDraft();
    }

    return {
        cancelEditing,
        editName,
        editRows,
        handleAddRow,
        handleDirectionChange,
        handleRemovePanel,
        handleRemoveRow,
        handleSave,
        handleUpdatePanel,
        isEditing,
        isSaving,
        setEditName,
        setIsEditing
    };
}
