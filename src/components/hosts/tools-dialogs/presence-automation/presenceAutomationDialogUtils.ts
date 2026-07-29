import { accessTypeLocaleKeyMap } from '@/shared/constants/accessType';
import type { FavoriteGroup } from '@/state/favoriteStoreTypes';

export type PresenceRuleActions = Record<string, unknown> & {
    status?: string;
    statusDescription?: string;
};

export type PresenceRuleCondition = Record<string, unknown> & {
    type: string;
};

export type TimeWindowCondition = PresenceRuleCondition & {
    type: 'timeWindow';
    days: number[];
    end: string;
    start: string;
    timezone: string;
};

export type PresenceAutomationRule = Record<string, unknown> & {
    actions?: PresenceRuleActions;
    conditions?: PresenceRuleCondition[];
    domain?: string;
    enabled?: boolean;
    id: string;
    label?: string;
    priority?: number;
    restorePreviousState?: boolean;
};

export type TimeAutomationRule = PresenceAutomationRule & {
    domain: 'time';
    conditions: PresenceRuleCondition[];
};

export type ContextAutomationRule = PresenceAutomationRule & {
    domain: 'context';
    friendCountValue?: number;
    playerCountValue?: number;
    preset?: string;
    selectedGroups?: string[];
    selectedInstanceTypes?: string[];
    selectedWorldGroups?: string[];
    specificFriendIds?: string[];
};

export type PresenceOption = {
    label: string;
    value: string;
};

type TranslationFunction = (key: string) => string;

export const dayOptions = [
    { value: 1, labelKey: 'common.days.monday' },
    { value: 2, labelKey: 'common.days.tuesday' },
    { value: 3, labelKey: 'common.days.wednesday' },
    { value: 4, labelKey: 'common.days.thursday' },
    { value: 5, labelKey: 'common.days.friday' },
    { value: 6, labelKey: 'common.days.saturday' },
    { value: 7, labelKey: 'common.days.sunday' }
] as const;

export const contextPresetOptions = [
    {
        value: 'alone',
        labelKey: 'view.tools.social_automation.preset_alone'
    },
    {
        value: 'withAnyone',
        labelKey: 'view.tools.social_automation.preset_with_anyone'
    },
    {
        value: 'withAnyFriend',
        labelKey: 'view.tools.social_automation.preset_with_any_friend'
    },
    {
        value: 'friendCountAtLeast',
        labelKey: 'view.tools.social_automation.preset_friend_count_at_least'
    },
    {
        value: 'playerCountAtLeast',
        labelKey: 'view.tools.social_automation.preset_player_count_at_least'
    },
    {
        value: 'withSelectedGroups',
        labelKey: 'view.tools.social_automation.preset_with_selected_groups'
    },
    {
        value: 'withSelectedFriend',
        labelKey: 'view.tools.social_automation.preset_with_selected_friend'
    },
    {
        value: 'inSelectedInstanceTypes',
        labelKey: 'view.tools.social_automation.preset_in_selected_room_types'
    },
    {
        value: 'inFavoriteWorlds',
        labelKey: 'view.tools.social_automation.preset_in_favorite_worlds'
    }
] as const;

export const priorityOptions = [
    {
        value: 'high',
        labelKey: 'view.tools.social_automation.priority_high',
        priority: 700
    },
    {
        value: 'medium',
        labelKey: 'view.tools.social_automation.priority_medium',
        priority: 400
    },
    {
        value: 'low',
        labelKey: 'view.tools.social_automation.priority_low',
        priority: 100
    }
] as const;

type PriorityValue = (typeof priorityOptions)[number]['value'];

function asRuleRecord(value: unknown): Record<string, unknown> {
    return value && typeof value === 'object'
        ? (value as Record<string, unknown>)
        : {};
}

function asStringArray(value: unknown): string[] {
    return Array.isArray(value) ? (value as string[]) : [];
}

export function priorityValueFromNumber(
    priority: unknown,
    fallback = 'medium'
): PriorityValue | string {
    const numericPriority = Number(priority);
    if (!Number.isFinite(numericPriority)) {
        return fallback;
    }
    if (numericPriority >= 600) {
        return 'high';
    }
    if (numericPriority >= 300) {
        return 'medium';
    }
    return 'low';
}

export function priorityLabelKeyFromNumber(
    priority: unknown,
    fallback = 'medium'
) {
    const value = priorityValueFromNumber(priority, fallback);
    return (
        priorityOptions.find((option) => option.value === value)?.labelKey ||
        priorityOptions[1].labelKey
    );
}

export function priorityNumberFromValue(
    value: unknown,
    fallback = 400
): number {
    return (
        priorityOptions.find((option) => option.value === value)?.priority ||
        fallback
    );
}

export function contextPresetLabelKeyFromValue(value: unknown) {
    return (
        contextPresetOptions.find((option) => option.value === value)
            ?.labelKey || 'view.tools.social_automation.preset_custom'
    );
}

export function createInstanceOptions(
    instanceTypes: readonly string[],
    t: TranslationFunction
): PresenceOption[] {
    return instanceTypes.map((type) => {
        const mapKey = type === 'groupOnly' ? 'groupMembers' : type;
        const localeKey = accessTypeLocaleKeyMap[mapKey];
        const isGroupAccessType =
            mapKey === 'groupPublic' ||
            mapKey === 'groupPlus' ||
            mapKey === 'groupMembers';
        if (!isGroupAccessType) {
            return { value: type, label: localeKey ? t(localeKey) : type };
        }
        const groupLabel = t(accessTypeLocaleKeyMap.group);
        const typeLabel = t(localeKey);
        return {
            value: type,
            label: typeLabel.toLowerCase().startsWith(groupLabel.toLowerCase())
                ? typeLabel
                : `${groupLabel} ${typeLabel}`
        };
    });
}

export function createGroupOptions({
    remoteGroups,
    localGroups
}: {
    remoteGroups?: FavoriteGroup[];
    localGroups?: string[];
}): PresenceOption[] {
    const remoteGroupOptions = (remoteGroups || []).map((group) => ({
        value: group.key || '',
        label: group.displayName || group.name || group.key || ''
    }));
    const localGroupOptions = (localGroups || []).map((group) => ({
        value: `local:${group}`,
        label: group
    }));
    return [...remoteGroupOptions, ...localGroupOptions].filter(
        (group) => group.value
    );
}

export function createTimeRule(label = ''): TimeAutomationRule {
    const days: number[] = [];

    return {
        id: `time-${Date.now()}`,
        enabled: true,
        domain: 'time',
        priority: 700,
        label,
        restorePreviousState: true,
        conditions: [
            {
                type: 'timeWindow',
                start: '21:00',
                end: '02:00',
                days,
                timezone: 'local'
            }
        ],
        actions: {}
    };
}

export function getTimeWindow(rule: PresenceAutomationRule) {
    return (rule.conditions?.find(
        (condition) => condition.type === 'timeWindow'
    ) || {
        type: 'timeWindow',
        start: '21:00',
        end: '02:00',
        days: [],
        timezone: 'local'
    }) as TimeWindowCondition;
}

export function shouldRestorePreviousState(rule: PresenceAutomationRule) {
    return rule?.restorePreviousState !== false;
}

export function hasGameRunningCondition(rule: PresenceAutomationRule) {
    return Boolean(
        rule.conditions?.some(
            (condition) =>
                condition?.type === 'isGameRunning' && condition.value !== false
        )
    );
}

export function setGameRunningCondition<TRule extends PresenceAutomationRule>(
    rule: TRule,
    enabled: boolean
): TRule {
    const otherConditions = (rule.conditions || []).filter(
        (condition) => condition?.type !== 'isGameRunning'
    );
    return {
        ...rule,
        conditions: enabled
            ? [{ type: 'isGameRunning' }, ...otherConditions]
            : otherConditions
    } as TRule;
}

export function buildContextConditions(rule: ContextAutomationRule) {
    const conditions: PresenceRuleCondition[] = [{ type: 'isGameRunning' }];
    if (rule.preset === 'alone') {
        conditions.push({ type: 'isAlone' });
    } else if (rule.preset === 'withAnyone') {
        conditions.push({ type: 'withCompany' });
    } else if (rule.preset === 'withAnyFriend') {
        conditions.push({ type: 'hasAnyFriend' });
    } else if (rule.preset === 'friendCountAtLeast') {
        conditions.push({
            type: 'friendCount',
            op: '>=',
            value: Number(rule.friendCountValue) || 1
        });
    } else if (rule.preset === 'playerCountAtLeast') {
        conditions.push({
            type: 'playerCount',
            op: '>=',
            value: Number(rule.playerCountValue) || 1
        });
    } else if (rule.preset === 'withSelectedGroups') {
        conditions.push({
            type: 'hasFriendInGroups',
            values: rule.selectedGroups || []
        });
    } else if (rule.preset === 'withSelectedFriend') {
        conditions.push({
            type: 'hasSpecificFriend',
            values: rule.specificFriendIds || []
        });
    } else if (rule.preset === 'inFavoriteWorlds') {
        conditions.push({
            type: 'worldInFavoriteGroups',
            values: rule.selectedWorldGroups || []
        });
    }

    if (rule.selectedInstanceTypes?.length) {
        conditions.push({
            type: 'instanceTypeIn',
            values: rule.selectedInstanceTypes || []
        });
    }
    return conditions;
}

export function createContextRule(label = ''): ContextAutomationRule {
    const rule: ContextAutomationRule = {
        id: `context-${Date.now()}`,
        enabled: true,
        domain: 'context',
        priority: 400,
        label,
        preset: 'alone',
        selectedGroups: [],
        selectedInstanceTypes: ['public', 'friends+'],
        selectedWorldGroups: [],
        specificFriendIds: [],
        friendCountValue: 1,
        playerCountValue: 1,
        actions: {
            status: 'join me'
        }
    };
    return {
        ...rule,
        conditions: buildContextConditions(rule)
    };
}

export function normalizeContextRule(rule: unknown): ContextAutomationRule {
    const source = asRuleRecord(rule);
    const normalized: ContextAutomationRule = {
        ...source,
        id: String(source.id || `context-${Date.now()}`),
        domain: 'context',
        preset: String(source.preset || 'alone'),
        selectedGroups: asStringArray(source.selectedGroups),
        selectedInstanceTypes: asStringArray(source.selectedInstanceTypes),
        selectedWorldGroups: asStringArray(source.selectedWorldGroups),
        specificFriendIds: asStringArray(source.specificFriendIds),
        friendCountValue: Number(source.friendCountValue) || 1,
        playerCountValue: Number(source.playerCountValue) || 1,
        actions: asRuleRecord(source.actions) as PresenceRuleActions
    };
    return {
        ...normalized,
        conditions: buildContextConditions(normalized)
    };
}

export function updateRule<TRule extends PresenceAutomationRule>(
    rules: readonly TRule[],
    ruleId: string,
    updater: (rule: TRule) => TRule
): TRule[] {
    return rules.map((rule) => {
        if (rule.id !== ruleId) {
            return rule;
        }
        return updater(rule);
    });
}
