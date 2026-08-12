import type { DashboardPanel } from '@/repositories/dashboardRepository';

import {
    createDashboardWidgetPanelValue,
    getDashboardPanelConfig
} from './dashboardConfig';
import type { DashboardPanelDefinition } from './dashboardRegistry';
import {
    getDashboardPanelDefinition,
    resolveDashboardPanelKey
} from './dashboardRegistry';

export type DashboardPageMetrics = {
    friendCount: number;
    onlineCount: number;
    favoriteFriendCount: number;
    favoriteWorldCount: number;
    favoriteAvatarCount: number;
    notificationCount: number;
};

export type DashboardPanelPreviewProps = {
    panelKey: string | null;
    definition: DashboardPanelDefinition | null;
    config: Record<string, unknown>;
    pageMetrics: DashboardPageMetrics;
    onConfigChange?: (nextConfig: Record<string, unknown>) => void;
};

export function createDashboardPanelPreviewProps({
    panel,
    pageMetrics,
    onPanelChange
}: {
    panel: DashboardPanel | null;
    pageMetrics: DashboardPageMetrics;
    onPanelChange?: (nextPanel: DashboardPanel | null) => void;
}): DashboardPanelPreviewProps {
    const panelKey = resolveDashboardPanelKey(panel);
    const definition = getDashboardPanelDefinition(panelKey);
    const config = getDashboardPanelConfig(panel) as Record<string, unknown>;
    const onConfigChange =
        definition?.category === 'widget' && onPanelChange
            ? (nextConfig: Record<string, unknown>) =>
                  onPanelChange(
                      createDashboardWidgetPanelValue(
                          definition.key,
                          nextConfig
                      )
                  )
            : undefined;

    return {
        panelKey,
        definition,
        config,
        pageMetrics,
        onConfigChange
    };
}
