import type { NavMenuItem } from '../navMenuModel';

export type NavEntryHandler = (entry: NavMenuItem) => void | Promise<void>;

export type NavMenuActionHandlers = {
    onSelect: NavEntryHandler;
    onEditDashboard: NavEntryHandler;
    onDeleteDashboard: NavEntryHandler;
    onUnpinTool: NavEntryHandler;
    onMarkAllRead: () => void | Promise<void>;
    onOpenCustomNav: () => void;
};
