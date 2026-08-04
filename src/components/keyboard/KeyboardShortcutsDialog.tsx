import { useTranslation } from 'react-i18next';

import { KeyboardShortcut } from '@/components/keyboard/KeyboardShortcut';
import { useRuntimeStore } from '@/state/runtimeStore';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle
} from '@/ui/shadcn/dialog';

const SHORTCUT_GROUPS = [
    {
        titleKey: 'shortcuts.group.general',
        items: [
            { labelKey: 'app_menu.quick_search', keys: ['K'] },
            { labelKey: 'app_menu.settings', keys: [','] },
            { labelKey: 'prompt.direct_access_omni.header', keys: ['D'] },
            { labelKey: 'app_menu.keyboard_shortcuts', keys: ['/'] }
        ]
    },
    {
        titleKey: 'shortcuts.group.layout',
        items: [
            { labelKey: 'nav_tooltip.collapse_nav', keys: ['B'] },
            { labelKey: 'app_menu.hide_friends_sidebar', keys: ['Shift', 'B'] }
        ]
    }
] as const;

export function KeyboardShortcutsDialog({
    open,
    onOpenChange
}: {
    open: boolean;
    onOpenChange(open: boolean): void;
}) {
    const { t } = useTranslation();
    const isMacHost = useRuntimeStore(
        (state) => state.hostCapabilities.platform === 'macos'
    );
    const modifier = isMacHost ? 'Meta' : 'Mod';

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>{t('shortcuts.title')}</DialogTitle>
                </DialogHeader>
                <div className="space-y-5">
                    {SHORTCUT_GROUPS.map((group) => (
                        <section key={group.titleKey}>
                            <h3 className="text-muted-foreground mb-2 text-xs font-medium uppercase">
                                {t(group.titleKey)}
                            </h3>
                            <ul className="divide-border divide-y">
                                {group.items.map((item) => (
                                    <li
                                        key={item.labelKey}
                                        className="flex items-center justify-between gap-4 py-2 text-sm"
                                    >
                                        <span className="min-w-0 truncate">
                                            {t(item.labelKey)}
                                        </span>
                                        <KeyboardShortcut
                                            keys={[modifier, ...item.keys]}
                                        />
                                    </li>
                                ))}
                            </ul>
                        </section>
                    ))}
                </div>
            </DialogContent>
        </Dialog>
    );
}
