import {
    LayoutDashboardIcon,
    PlusIcon,
    SaveIcon,
    Trash2Icon,
    XIcon
} from 'lucide-react';
import { Fragment } from 'react';
import { useTranslation } from 'react-i18next';

import { PageScaffold } from '@/components/layout/PageScaffold';
import { userFacingErrorMessage } from '@/lib/errorDisplay';
import { Button } from '@/ui/shadcn/button';
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle
} from '@/ui/shadcn/card';
import { Input } from '@/ui/shadcn/input';
import {
    ResizableHandle,
    ResizablePanel,
    ResizablePanelGroup
} from '@/ui/shadcn/resizable';

import { DashboardAddRowControl } from './components/DashboardAddRowControl';
import {
    DashboardEditorRow,
    DashboardReadRow
} from './components/DashboardViewParts';
import { getDashboardRowKey } from './dashboardConfig';
import { useDashboardPageController } from './useDashboardPageController';

export function DashboardPage() {
    const { t } = useTranslation();
    const {
        actions,
        dashboard,
        dashboardLayout,
        dashboards,
        detail,
        editor,
        id,
        loaded,
        loadStatus
    } = useDashboardPageController();

    if (!loaded && loadStatus !== 'error') {
        return (
            <PageScaffold className="gap-6">
                <Card>
                    <CardHeader>
                        <CardTitle className="flex items-center gap-2">
                            <LayoutDashboardIcon className="size-5" />
                            {t('dashboard.default_name')}
                        </CardTitle>
                        <CardDescription>
                            {t(
                                'view.dashboard.loading.loading_dashboard_configuration'
                            )}
                        </CardDescription>
                    </CardHeader>
                </Card>
            </PageScaffold>
        );
    }

    if (!dashboard) {
        return (
            <PageScaffold className="gap-6">
                <Card>
                    <CardHeader className="gap-4">
                        <div className="flex flex-col gap-2">
                            <CardTitle className="flex items-center gap-2">
                                <LayoutDashboardIcon className="size-5" />
                                {t('dashboard.default_name')}
                            </CardTitle>
                            <CardDescription>
                                {dashboards.length
                                    ? t(
                                          'view.dashboard.empty.that_dashboard_no_longer_exists_in_the_stored_config'
                                      )
                                    : t(
                                          'view.dashboard.empty.no_dashboard_definitions_are_stored_yet'
                                      )}
                            </CardDescription>
                        </div>
                        {detail ? (
                            <div className="text-muted-foreground text-sm">
                                {userFacingErrorMessage(
                                    detail,
                                    t(
                                        'view.dashboard.toast.failed_to_load_dashboard_configuration'
                                    )
                                )}
                            </div>
                        ) : null}
                    </CardHeader>
                    <CardContent className="flex flex-wrap gap-2">
                        <Button
                            type="button"
                            onClick={actions.createNewDashboard}
                        >
                            <PlusIcon data-icon="inline-start" />
                            {t('dashboard.new_dashboard')}
                        </Button>
                        {dashboards.length ? (
                            <Button
                                type="button"
                                variant="outline"
                                onClick={actions.openFirstDashboard}
                            >
                                {t(
                                    'view.dashboard.action.open_first_dashboard'
                                )}
                            </Button>
                        ) : (
                            <Button
                                type="button"
                                variant="outline"
                                onClick={actions.goToFeed}
                            >
                                {t('view.dashboard.action.back_to_feed')}
                            </Button>
                        )}
                    </CardContent>
                </Card>
            </PageScaffold>
        );
    }

    const rowCount = dashboard.rows?.length || 0;

    return (
        <PageScaffold className="gap-3">
            {editor.isEditing ? (
                <div className="bg-card flex items-center gap-2 rounded-md border px-3 py-2">
                    <Input
                        value={editor.editName}
                        onChange={(event) =>
                            editor.setEditName(event.target.value)
                        }
                        placeholder={t('view.dashboard.label.dashboard_name')}
                        className="mx-2 h-7 max-w-52 text-sm"
                    />
                    <div className="flex gap-2">
                        <Button
                            type="button"
                            variant="secondary"
                            size="sm"
                            onClick={editor.cancelEditing}
                        >
                            <XIcon data-icon="inline-start" />
                            {t('common.actions.cancel')}
                        </Button>
                        <Button
                            type="button"
                            variant="destructive"
                            size="sm"
                            onClick={actions.deleteCurrentDashboard}
                        >
                            <Trash2Icon data-icon="inline-start" />
                            {t('common.actions.delete')}
                        </Button>
                    </div>
                    <Button
                        type="button"
                        className="ml-auto"
                        size="sm"
                        onClick={editor.handleSave}
                        disabled={editor.isSaving}
                    >
                        <SaveIcon data-icon="inline-start" />
                        {t('common.actions.save')}
                    </Button>
                </div>
            ) : null}
            <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
                {editor.isEditing ? (
                    <>
                        {editor.editRows.length ? (
                            editor.editRows.map((row, rowIndex) => (
                                <DashboardEditorRow
                                    key={`edit-row-${rowIndex}`}
                                    row={row}
                                    rowIndex={rowIndex}
                                    onPanelChange={(panelIndex, nextPanel) =>
                                        editor.handleUpdatePanel(
                                            rowIndex,
                                            panelIndex,
                                            nextPanel
                                        )
                                    }
                                    onPanelRemove={(panelIndex) =>
                                        editor.handleRemovePanel(
                                            rowIndex,
                                            panelIndex
                                        )
                                    }
                                    onRowRemove={() =>
                                        editor.handleRemoveRow(rowIndex)
                                    }
                                    onDirectionChange={(direction) =>
                                        editor.handleDirectionChange(
                                            rowIndex,
                                            direction
                                        )
                                    }
                                />
                            ))
                        ) : (
                            <div className="text-muted-foreground flex min-h-[180px] items-center justify-center rounded-md border border-dashed text-sm">
                                {t(
                                    'view.dashboard.action.add_a_row_to_start_building_this_dashboard'
                                )}
                            </div>
                        )}

                        <DashboardAddRowControl
                            onAddRow={editor.handleAddRow}
                        />
                    </>
                ) : rowCount ? (
                    <ResizablePanelGroup
                        id={`dashboard-${id}`}
                        orientation="vertical"
                        className="min-h-0 flex-1"
                        defaultLayout={dashboardLayout.defaultLayout}
                        onLayoutChanged={dashboardLayout.onLayoutChanged}
                    >
                        {dashboard.rows.map((row, rowIndex) => {
                            const rowKey = getDashboardRowKey(row);
                            return (
                                <Fragment key={`row-${rowKey}`}>
                                    <ResizablePanel
                                        id={`dashboard-${id}-row-panel-${rowKey}`}
                                        defaultSize={`${100 / rowCount}%`}
                                        minSize="10%"
                                    >
                                        <DashboardReadRow
                                            row={row}
                                            dashboardId={id}
                                            onPanelChange={(
                                                panelIndex,
                                                nextPanel
                                            ) => {
                                                actions.updateLivePanel(
                                                    rowIndex,
                                                    panelIndex,
                                                    nextPanel
                                                );
                                            }}
                                        />
                                    </ResizablePanel>
                                    {rowIndex < rowCount - 1 ? (
                                        <ResizableHandle />
                                    ) : null}
                                </Fragment>
                            );
                        })}
                    </ResizablePanelGroup>
                ) : (
                    <div className="text-muted-foreground flex flex-1 items-center justify-center rounded-md border border-dashed">
                        <div className="flex flex-col items-center gap-3">
                            <p>{t('dashboard.empty')}</p>
                            <Button
                                type="button"
                                onClick={() => editor.setIsEditing(true)}
                            >
                                {t('dashboard.actions.start_editing')}
                            </Button>
                        </div>
                    </div>
                )}
            </div>
        </PageScaffold>
    );
}
