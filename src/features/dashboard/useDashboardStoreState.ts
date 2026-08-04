import { useEffect } from 'react';
import { useParams } from 'react-router';

import { useDashboardStore } from '@/state/dashboardStore';

export function useDashboardStoreState() {
    const { id = '' } = useParams();
    const dashboards = useDashboardStore((state) => state.dashboards);
    const loaded = useDashboardStore((state) => state.loaded);
    const loadStatus = useDashboardStore((state) => state.loadStatus);
    const detail = useDashboardStore((state) => state.detail);
    const editingDashboardId = useDashboardStore(
        (state) => state.editingDashboardId
    );
    const ensureLoaded = useDashboardStore((state) => state.ensureLoaded);
    const consumeEditingDashboardId = useDashboardStore(
        (state) => state.consumeEditingDashboardId
    );
    const dashboard = dashboards.find((entry) => entry.id === id) || null;

    useEffect(() => {
        ensureLoaded().catch(() => {});
    }, [ensureLoaded]);

    return {
        consumeEditingDashboardId,
        dashboard,
        dashboards,
        detail,
        editingDashboardId,
        id,
        loaded,
        loadStatus
    };
}
