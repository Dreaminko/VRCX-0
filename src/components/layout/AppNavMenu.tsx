import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocation, useNavigate } from 'react-router';
import { toast } from 'sonner';

import type { Dashboard } from '@/repositories/dashboardRepository';
import { setNavbarCollapsedPreference } from '@/services/preferencesService';
import { triggerToolByKey } from '@/services/toolActionService';
import { DASHBOARD_NAV_KEY_PREFIX } from '@/shared/constants/dashboard';
import { useDashboardStore } from '@/state/dashboardStore';
import { useModalStore } from '@/state/modalStore';
import { usePreferencesStore } from '@/state/preferencesStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { useSessionStore } from '@/state/sessionStore';
import { useShellStore } from '@/state/shellStore';
import { useVrcNotificationStore } from '@/state/vrcNotificationStore';

import {
    isDashboardEntry,
    isEntryActive,
    isToolEntry,
    removeNavKeyFromLayout
} from './AppNavMenuParts';
import {
    AppNavCreateDashboardHeader,
    AppNavFooter,
    AppNavMenuContent
} from './AppNavMenuSections';
import type { CustomNavLayout } from './custom-nav-dialog/customNavLayout';
import { CustomNavDialog } from './CustomNavDialog';
import {
    getPathForNavEntry,
    loadNavMenuModel,
    NAV_CUSTOMIZE_REQUESTED_EVENT,
    NAV_LAYOUT_UPDATED_EVENT,
    routePathByName,
    saveNavMenuModel,
    type NavDefinition,
    type NavLayoutEntry,
    type NavMenuItem,
    type NavMenuModel
} from './navMenuModel';

type Translate = ReturnType<typeof useTranslation>['t'];
type Navigation = ReturnType<typeof useNavigate>;
type RouteLocation = ReturnType<typeof useLocation>;
type SaveAndApplyNavLayout = (
    layout: unknown,
    hiddenKeys: unknown
) => Promise<NavMenuModel>;

function resolveActiveIndex(menuItems: NavMenuItem[], pathname: string) {
    for (const item of menuItems) {
        if (item.children?.length) {
            const activeChild = item.children.find((entry) =>
                isEntryActive(entry, pathname)
            );
            if (activeChild) {
                return activeChild.index;
            }
            continue;
        }
        if (isEntryActive(item, pathname)) {
            return item.index;
        }
    }
    return '';
}

function useAppNavModel({
    dashboards,
    notificationLayout,
    preferencesHydrated,
    t
}: {
    dashboards: Dashboard[];
    notificationLayout: string;
    preferencesHydrated: boolean;
    t: Translate;
}) {
    const [menuItems, setMenuItems] = useState<NavMenuItem[]>([]);
    const [navLayout, setNavLayout] = useState<NavLayoutEntry[]>([]);
    const [navHiddenKeys, setNavHiddenKeys] = useState<string[]>([]);
    const [navDefinitions, setNavDefinitions] = useState<NavDefinition[]>([]);
    const [defaultNavLayout, setDefaultNavLayout] = useState<NavLayoutEntry[]>(
        []
    );

    function applyModel(model: NavMenuModel) {
        setNavLayout(model.layout);
        setNavHiddenKeys(model.hiddenKeys);
        setNavDefinitions(model.definitions);
        setDefaultNavLayout(model.defaultLayout);
        setMenuItems(model.menuItems);
    }

    useEffect(() => {
        if (!preferencesHydrated) {
            return undefined;
        }
        let active = true;
        async function loadModel() {
            const model = await loadNavMenuModel({
                dashboards: useDashboardStore.getState().dashboards,
                notificationLayout,
                t
            });
            if (!active || !model) {
                return;
            }
            applyModel(model);
        }

        loadModel().catch((error: unknown) => {
            console.warn('Failed to load navigation layout:', error);
            if (active) {
                setMenuItems([]);
            }
        });

        const handleNavLayoutUpdated = () => {
            loadModel().catch((error: unknown) => {
                console.warn('Failed to reload navigation layout:', error);
            });
        };
        window.addEventListener(
            NAV_LAYOUT_UPDATED_EVENT,
            handleNavLayoutUpdated
        );
        return () => {
            active = false;
            window.removeEventListener(
                NAV_LAYOUT_UPDATED_EVENT,
                handleNavLayoutUpdated
            );
        };
    }, [dashboards, notificationLayout, preferencesHydrated, t]);

    async function saveAndApplyNavLayout(
        nextLayout: unknown,
        nextHiddenKeys: unknown
    ) {
        const model = await saveNavMenuModel({
            layout: nextLayout,
            hiddenKeys: nextHiddenKeys,
            dashboards: useDashboardStore.getState().dashboards,
            notificationLayout,
            t
        });
        applyModel(model);
        return model;
    }

    return {
        defaultNavLayout,
        menuItems,
        navDefinitions,
        navHiddenKeys,
        navLayout,
        saveAndApplyNavLayout
    };
}

function useAppNavNotifications({
    activeIndex,
    currentUserId,
    sessionPhase,
    t
}: {
    activeIndex: string;
    currentUserId: string | null;
    sessionPhase: string;
    t: Translate;
}) {
    const notifiedMenus = useShellStore((state) => state.notifiedMenus);
    const removeNavNotification = useShellStore((state) => state.removeNotify);
    const vrcUnseenNotificationCount = useVrcNotificationStore(
        (state) => state.unseenCount
    );
    const markAllVrcNotificationsSeen = useVrcNotificationStore(
        (state) => state.markAllSeen
    );
    const loadVrcNotifications = useVrcNotificationStore(
        (state) => state.refreshForCurrentUser
    );
    const notifiedKeys = new Set(notifiedMenus);
    if (vrcUnseenNotificationCount > 0) {
        notifiedKeys.add('notification');
    }

    useEffect(() => {
        if (sessionPhase !== 'ready' || !currentUserId) {
            return;
        }
        loadVrcNotifications().catch(() => {});
    }, [currentUserId, loadVrcNotifications, sessionPhase]);

    useEffect(() => {
        if (!activeIndex) {
            return;
        }
        removeNavNotification(activeIndex);
    }, [activeIndex, removeNavNotification]);

    async function markAllRead() {
        const store = useVrcNotificationStore.getState();
        if (!store.unseenCount) {
            removeNavNotification('notification');
            return;
        }
        try {
            await markAllVrcNotificationsSeen();
            removeNavNotification('notification');
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.app_nav_menu.toast.failed_to_mark_notifications_as_seen'
                      )
            );
        }
    }

    return {
        hasNotifications: notifiedKeys.size > 0,
        markAllRead,
        notifiedKeys
    };
}

function useAppNavDashboardActions({
    location,
    navigate,
    t
}: {
    location: RouteLocation;
    navigate: Navigation;
    t: Translate;
}) {
    const createDashboard = useDashboardStore((state) => state.createDashboard);
    const deleteDashboard = useDashboardStore((state) => state.deleteDashboard);
    const setEditingDashboardId = useDashboardStore(
        (state) => state.setEditingDashboardId
    );
    const confirm = useModalStore((state) => state.confirm);
    const [isCreatingDashboard, setIsCreatingDashboard] = useState(false);

    async function createDashboardFromNav() {
        setIsCreatingDashboard(true);
        try {
            const dashboard = await createDashboard(
                t('dashboard.default_name')
            );
            setEditingDashboardId(dashboard.id);
            navigate(`/dashboard/${dashboard.id}`);
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.app_nav_menu.toast.failed_to_create_dashboard'
                      )
            );
        } finally {
            setIsCreatingDashboard(false);
        }
    }

    async function editDashboard(entry: NavMenuItem) {
        if (!isDashboardEntry(entry)) {
            return;
        }
        const dashboardId = String(entry.index || '').replace(
            DASHBOARD_NAV_KEY_PREFIX,
            ''
        );
        if (!dashboardId) {
            return;
        }
        setEditingDashboardId(dashboardId);
        if (location.pathname !== `/dashboard/${dashboardId}`) {
            navigate(`/dashboard/${dashboardId}`);
        }
    }

    async function deleteDashboardFromNav(entry: NavMenuItem) {
        if (!isDashboardEntry(entry)) {
            return;
        }
        const dashboardId = String(entry.index || '').replace(
            DASHBOARD_NAV_KEY_PREFIX,
            ''
        );
        if (!dashboardId) {
            return;
        }
        const result = await confirm({
            title: t('dashboard.confirmations.delete_title'),
            description: t('dashboard.confirmations.delete_description'),
            destructive: true
        });
        if (!result.ok) {
            return;
        }
        try {
            await deleteDashboard(dashboardId);
            if (location.pathname === `/dashboard/${dashboardId}`) {
                navigate('/feed', { replace: true });
            }
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.app_nav_menu.toast.failed_to_delete_dashboard'
                      )
            );
        }
    }

    return {
        createDashboardFromNav,
        deleteDashboardFromNav,
        editDashboard,
        isCreatingDashboard,
        setEditingDashboardId
    };
}

function useAppNavToolActions({
    navHiddenKeys,
    navLayout,
    saveAndApplyNavLayout,
    t
}: {
    navHiddenKeys: string[];
    navLayout: NavLayoutEntry[];
    saveAndApplyNavLayout: SaveAndApplyNavLayout;
    t: Translate;
}) {
    async function unpinToolEntry(entry: NavMenuItem) {
        if (!isToolEntry(entry)) {
            return;
        }
        try {
            const navKey = entry.index;
            await saveAndApplyNavLayout(
                removeNavKeyFromLayout(navLayout, navKey),
                navHiddenKeys
            );
            toast.success(t('nav_menu.custom_nav.unpinned'));
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.app_nav_menu.toast.failed_to_unpin_tool_from_navigation'
                      )
            );
        }
    }

    return { unpinToolEntry };
}

export function AppNavMenu({ isCollapsed }: { isCollapsed: boolean }) {
    const navigate = useNavigate();
    const location = useLocation();
    const { t } = useTranslation();
    const sidebarOpen = useShellStore((state) => state.sidebarOpen);
    const dashboards = useDashboardStore((state) => state.dashboards);
    const ensureDashboardsLoaded = useDashboardStore(
        (state) => state.ensureLoaded
    );
    const sessionPhase = useSessionStore((state) => state.sessionPhase);
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const preferencesHydrated = usePreferencesStore(
        (state) => state.preferencesHydrated
    );
    const notificationLayout = usePreferencesStore(
        (state) => state.notificationLayout
    );
    const [customNavDialogOpen, setCustomNavDialogOpen] = useState(false);
    const showNewDashboardButton = usePreferencesStore(
        (state) => state.showNewDashboardButton
    );
    const {
        defaultNavLayout,
        menuItems,
        navDefinitions,
        navHiddenKeys,
        navLayout,
        saveAndApplyNavLayout
    } = useAppNavModel({
        dashboards,
        notificationLayout,
        preferencesHydrated,
        t
    });
    const activeIndex = resolveActiveIndex(menuItems, location.pathname);
    const { hasNotifications, markAllRead, notifiedKeys } =
        useAppNavNotifications({
            activeIndex,
            currentUserId,
            sessionPhase,
            t
        });
    const {
        createDashboardFromNav,
        deleteDashboardFromNav,
        editDashboard,
        isCreatingDashboard,
        setEditingDashboardId
    } = useAppNavDashboardActions({ location, navigate, t });
    const { unpinToolEntry } = useAppNavToolActions({
        navHiddenKeys,
        navLayout,
        saveAndApplyNavLayout,
        t
    });

    useEffect(() => {
        ensureDashboardsLoaded().catch(() => {});
    }, [ensureDashboardsLoaded]);

    useEffect(() => {
        const handleCustomizeRequested = () => {
            setCustomNavDialogOpen(true);
        };
        window.addEventListener(
            NAV_CUSTOMIZE_REQUESTED_EVENT,
            handleCustomizeRequested
        );
        return () => {
            window.removeEventListener(
                NAV_CUSTOMIZE_REQUESTED_EVENT,
                handleCustomizeRequested
            );
        };
    }, []);

    const shouldShowCreateDashboard = showNewDashboardButton;

    async function handleSelectEntry(entry: NavMenuItem) {
        if (!entry) {
            return;
        }
        if (entry.action?.type === 'tool') {
            await triggerToolByKey(entry.action.toolKey, { navigate, t });
            return;
        }
        const path = getPathForNavEntry(entry);
        if (path) {
            navigate(path);
        }
    }

    async function handleCustomNavSave(
        nextLayout: CustomNavLayout,
        nextHiddenKeys: unknown[]
    ) {
        try {
            await saveAndApplyNavLayout(nextLayout, nextHiddenKeys);
            setCustomNavDialogOpen(false);
            toast.success(t('message.update_success'));
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.app_nav_menu.toast.failed_to_save_custom_navigation'
                      )
            );
        }
    }

    async function handleDashboardCreatedFromCustomNav(
        dashboardId: string,
        nextLayout: CustomNavLayout,
        nextHiddenKeys: unknown[]
    ) {
        try {
            await saveAndApplyNavLayout(nextLayout, nextHiddenKeys);
            setCustomNavDialogOpen(false);
            setEditingDashboardId(dashboardId);
            navigate(`/dashboard/${dashboardId}`);
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.app_nav_menu.toast.failed_to_save_dashboard_navigation'
                      )
            );
        }
    }

    return (
        <>
            <AppNavCreateDashboardHeader
                visible={shouldShowCreateDashboard}
                disabled={isCreatingDashboard}
                onCreateDashboard={createDashboardFromNav}
            />

            <AppNavMenuContent
                menuItems={menuItems}
                isCollapsed={isCollapsed}
                activeIndex={activeIndex}
                pathname={location.pathname}
                notifiedKeys={notifiedKeys}
                hasNotifications={hasNotifications}
                onSelect={handleSelectEntry}
                onMarkAllRead={markAllRead}
                onCreateDashboard={createDashboardFromNav}
                onEditDashboard={editDashboard}
                onDeleteDashboard={deleteDashboardFromNav}
                onUnpinTool={unpinToolEntry}
                onOpenCustomNav={() => setCustomNavDialogOpen(true)}
            />

            <AppNavFooter
                sidebarOpen={sidebarOpen}
                onNavigateSettings={() => navigate(routePathByName.settings)}
                onToggleSidebar={() =>
                    setNavbarCollapsedPreference(sidebarOpen)
                }
            />
            <CustomNavDialog
                open={customNavDialogOpen}
                layout={navLayout}
                hiddenKeys={navHiddenKeys}
                defaultLayout={defaultNavLayout}
                defaultHiddenKeys={[]}
                definitions={navDefinitions}
                onOpenChange={setCustomNavDialogOpen}
                onSave={handleCustomNavSave}
                onDashboardCreated={handleDashboardCreatedFromCustomNav}
            />
        </>
    );
}
