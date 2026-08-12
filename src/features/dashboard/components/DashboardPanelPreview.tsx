import { ArrowRightIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router';

import { cn } from '@/lib/utils';
import { Button } from '@/ui/shadcn/button';

import { canEmbedDashboardPagePanel } from '../dashboardPagePanelRegistry';
import type {
    DashboardPageMetrics,
    DashboardPanelPreviewProps
} from '../dashboardPanelPreviewModel';
import { DashboardEmbeddedPagePanel } from './DashboardEmbeddedPagePanel';
import { DashboardFeedWidget } from './widgets/DashboardFeedWidget';
import { DashboardGameLogWidget } from './widgets/DashboardGameLogWidget';
import { DashboardInstanceWidget } from './widgets/DashboardInstanceWidget';

export type DashboardPanelFrameMode = 'card' | 'docked';

type DashboardPanelPreviewComponentProps = DashboardPanelPreviewProps & {
    frameMode?: DashboardPanelFrameMode;
};

type PreviewMetricProps = {
    label: string;
    value: number | string;
};

type DashboardWidgetPreviewProps = {
    definition: NonNullable<DashboardPanelPreviewProps['definition']>;
    config: Record<string, unknown>;
    onConfigChange?: DashboardPanelPreviewProps['onConfigChange'];
};

function PreviewMetric({ label, value }: PreviewMetricProps) {
    return (
        <div className="bg-muted/20 rounded-md border px-3 py-2">
            <div className="text-muted-foreground text-xs tracking-wide uppercase">
                {label}
            </div>
            <div className="text-sm font-medium">{value}</div>
        </div>
    );
}

function DashboardWidgetPreview({
    definition,
    config,
    onConfigChange
}: DashboardWidgetPreviewProps) {
    if (definition.key === 'widget:instance') {
        return (
            <DashboardInstanceWidget
                config={config}
                configUpdater={onConfigChange ?? null}
            />
        );
    }

    if (definition.key === 'widget:game-log') {
        return (
            <DashboardGameLogWidget
                config={config}
                configUpdater={onConfigChange ?? null}
            />
        );
    }

    return (
        <DashboardFeedWidget
            config={config}
            configUpdater={onConfigChange ?? null}
        />
    );
}

function DashboardPagePreview({
    definition,
    pageMetrics
}: {
    definition: NonNullable<DashboardPanelPreviewProps['definition']>;
    pageMetrics: DashboardPageMetrics;
}) {
    const { t } = useTranslation();

    const navigate = useNavigate();

    let metrics = [
        <PreviewMetric
            key="status"
            label={t('view.dashboard.label.status')}
            value={t('view.dashboard.label.route_available')}
        />
    ];

    if (definition.key === 'friend-list') {
        metrics = [
            <PreviewMetric
                key="friends"
                label={t('view.dashboard.label.friends')}
                value={pageMetrics.friendCount}
            />,
            <PreviewMetric
                key="online"
                label={t('view.dashboard.label.online')}
                value={pageMetrics.onlineCount}
            />
        ];
    } else if (definition.key === 'favorite-friends') {
        metrics = [
            <PreviewMetric
                key="favorites"
                label={t('view.dashboard.label.favorites')}
                value={pageMetrics.favoriteFriendCount}
            />
        ];
    } else if (definition.key === 'favorite-worlds') {
        metrics = [
            <PreviewMetric
                key="favorites"
                label={t('view.dashboard.label.favorites')}
                value={pageMetrics.favoriteWorldCount}
            />
        ];
    } else if (definition.key === 'favorite-avatars') {
        metrics = [
            <PreviewMetric
                key="favorites"
                label={t('view.dashboard.label.favorites')}
                value={pageMetrics.favoriteAvatarCount}
            />
        ];
    } else if (definition.key === 'notification') {
        metrics = [
            <PreviewMetric
                key="notifications"
                label={t('view.dashboard.label.notifications')}
                value={pageMetrics.notificationCount}
            />
        ];
    }

    return (
        <div className="flex h-full flex-col gap-4">
            <div className="grid gap-3 sm:grid-cols-2">{metrics}</div>
            <div className="bg-muted/10 text-muted-foreground mt-auto flex items-center justify-between gap-3 rounded-md border px-3 py-2 text-sm">
                <span>{definition.path}</span>
                <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                        if (definition.path) {
                            navigate(definition.path);
                        }
                    }}
                >
                    {t('common.actions.open')}
                    <ArrowRightIcon data-icon="inline-end" />
                </Button>
            </div>
        </div>
    );
}

export function DashboardPanelPreview({
    panelKey,
    definition,
    config,
    pageMetrics,
    onConfigChange,
    frameMode = 'card'
}: DashboardPanelPreviewComponentProps) {
    const { t } = useTranslation();

    const canEmbedPagePanel =
        definition?.category === 'page' && canEmbedDashboardPagePanel(panelKey);
    const frameClassName = cn(
        'bg-card relative h-full min-h-[180px] overflow-hidden',
        frameMode === 'docked' ? 'rounded-none border-0' : 'rounded-md border'
    );
    const placeholderFrameClassName = cn(
        frameClassName,
        'text-muted-foreground flex items-center justify-center text-sm',
        frameMode === 'card' && 'border-dashed'
    );

    if (!panelKey) {
        return (
            <div className={placeholderFrameClassName}>
                <div className="py-10 text-center">
                    {t('view.dashboard.label.panel_not_configured')}
                </div>
            </div>
        );
    }

    if (!definition) {
        return (
            <div className={placeholderFrameClassName}>
                {t('view.dashboard.error.unsupported_panel')} {panelKey}
            </div>
        );
    }

    if (canEmbedPagePanel) {
        return (
            <div
                className={cn(
                    frameClassName,
                    'dashboard-panel is-compact-table flex'
                )}
            >
                <div className="h-full w-full overflow-y-auto">
                    <DashboardEmbeddedPagePanel panelKey={panelKey} />
                </div>
            </div>
        );
    }

    if (definition.category === 'widget') {
        return (
            <div
                className={cn(
                    frameClassName,
                    'dashboard-panel is-compact-table flex'
                )}
            >
                <div className="h-full w-full overflow-y-auto">
                    <DashboardWidgetPreview
                        definition={definition}
                        config={config}
                        onConfigChange={onConfigChange}
                    />
                </div>
            </div>
        );
    }

    return (
        <div className={cn(frameClassName, 'flex p-3')}>
            <DashboardPagePreview
                definition={definition}
                pageMetrics={pageMetrics}
            />
        </div>
    );
}
