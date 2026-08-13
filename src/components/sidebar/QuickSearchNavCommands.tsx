import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

import { getNavIconComponent } from '@/components/layout/navIconRegistry';
import { getPathForNavEntry } from '@/components/layout/navMenuModel';
import { isToolNavKey, toolDefinitions } from '@/shared/constants/tools';
import { navDefinitions } from '@/shared/constants/ui';
import { CommandGroup, CommandItem } from '@/ui/shadcn/command';

const NAV_RESULT_LIMIT = 6;

export type QuickSearchNavCommand = {
    key: string;
    label: string;
    icon: unknown;
    keywords: string;
    target: { type: 'path'; path: string } | { type: 'tool'; toolKey: string };
};

function isNavCommand(
    command: QuickSearchNavCommand | null
): command is QuickSearchNavCommand {
    return Boolean(command);
}

export function useNavCommands(normalizedQuery: string) {
    const { t } = useTranslation();

    const allCommands = useMemo(() => {
        const pageCommands = navDefinitions
            .filter((definition) => !isToolNavKey(definition.key))
            .map((definition): QuickSearchNavCommand | null => {
                const path = getPathForNavEntry(definition);
                if (!path) {
                    return null;
                }
                const label = t(definition.labelKey);
                const keywords =
                    `${label} ${definition.key} ${path}`.toLowerCase();
                return {
                    key: definition.key,
                    label,
                    icon: definition.icon,
                    keywords,
                    target: { type: 'path', path }
                };
            })
            .filter(isNavCommand);
        const toolCommands = toolDefinitions.map(
            (tool): QuickSearchNavCommand => {
                const label = t(tool.titleKey);
                const description = t(tool.descriptionKey);
                return {
                    key: `tool-${tool.key}`,
                    label,
                    icon: tool.navIcon,
                    keywords:
                        `${label} ${description} ${tool.key} ${tool.category}`.toLowerCase(),
                    target: { type: 'tool', toolKey: tool.key }
                };
            }
        );
        return [...pageCommands, ...toolCommands];
    }, [t]);

    return useMemo(() => {
        if (!normalizedQuery) {
            return [];
        }
        return allCommands
            .filter((command) => command.keywords.includes(normalizedQuery))
            .slice(0, NAV_RESULT_LIMIT);
    }, [allCommands, normalizedQuery]);
}

export function NavResultGroup({
    title,
    items,
    onSelect
}: {
    title: string;
    items: readonly QuickSearchNavCommand[];
    onSelect: (item: QuickSearchNavCommand) => void;
}) {
    if (!items.length) {
        return null;
    }
    return (
        <CommandGroup heading={title}>
            {items.map((item) => {
                const Icon = getNavIconComponent(item.icon);
                return (
                    <CommandItem
                        key={`nav:${item.key}`}
                        value={item.keywords}
                        className="gap-3"
                        onSelect={() => onSelect(item)}
                    >
                        <Icon className="size-4 shrink-0" />
                        <span className="min-w-0 flex-1 truncate">
                            {item.label}
                        </span>
                    </CommandItem>
                );
            })}
        </CommandGroup>
    );
}
