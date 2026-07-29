import type { TFunction } from 'i18next';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { userStatusLabel } from '@/shared/utils/userStatus';
import {
    Empty,
    EmptyDescription,
    EmptyHeader,
    EmptyTitle
} from '@/ui/shadcn/empty';
import {
    Field,
    FieldDescription,
    FieldGroup,
    FieldLabel,
    FieldLegend,
    FieldSet
} from '@/ui/shadcn/field';
import { Input } from '@/ui/shadcn/input';
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';

import {
    AutomationSplitLayout,
    CompactCheckList,
    RuleEditorPanel,
    RuleList,
    RuleListItem,
    RuleSummaryBadge
} from './AutomationRuleLayout';
import {
    contextPresetLabelKeyFromValue,
    contextPresetOptions,
    createContextRule,
    normalizeContextRule,
    type ContextAutomationRule,
    type PresenceOption,
    type PresenceRuleActions,
    priorityLabelKeyFromNumber,
    priorityNumberFromValue,
    priorityOptions,
    priorityValueFromNumber,
    updateRule
} from './presenceAutomationDialogUtils';
import { PresenceRuleActionFields } from './PresenceRuleActionFields';

const I18N_ROOT = 'view.tools.social_automation';

type ContextRulesTabProps = {
    contextRules: ContextAutomationRule[];
    groupOptions: PresenceOption[];
    instanceOptions: PresenceOption[];
    loading: boolean;
    onRulesChange: (rules: ContextAutomationRule[]) => unknown;
    worldGroupOptions: PresenceOption[];
};

function hasAction(rule: ContextAutomationRule, key: string) {
    return Object.prototype.hasOwnProperty.call(rule.actions || {}, key);
}

function updateAction(
    rule: ContextAutomationRule,
    patch: Partial<PresenceRuleActions>
): ContextAutomationRule {
    return {
        ...rule,
        actions: {
            ...(rule.actions || {}),
            ...patch
        }
    };
}

function removeAction(rule: ContextAutomationRule, key: string) {
    const actions: PresenceRuleActions = { ...(rule.actions || {}) };
    delete actions[key];
    return {
        ...rule,
        actions
    };
}

function parseUserIds(value: unknown) {
    return String(value || '')
        .split(',')
        .map((entry) => entry.trim())
        .filter(Boolean);
}

function ruleTitle(rule: ContextAutomationRule, t: TFunction) {
    return rule?.label || t(`${I18N_ROOT}.room_rule_default`);
}

function actionSummary(rule: ContextAutomationRule, t: TFunction) {
    const parts = [];
    if (rule.actions?.status) {
        parts.push(userStatusLabel(rule.actions.status, t));
    }
    if (hasAction(rule, 'statusDescription')) {
        parts.push(t(`${I18N_ROOT}.signature`));
    }
    return parts.length ? parts.join(' / ') : t(`${I18N_ROOT}.do_not_change`);
}

export function ContextRulesTab({
    loading,
    groupOptions,
    worldGroupOptions,
    instanceOptions,
    contextRules,
    onRulesChange
}: ContextRulesTabProps) {
    const { t } = useTranslation();
    const rules = Array.isArray(contextRules) ? contextRules : [];
    const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null);

    useEffect(() => {
        if (!rules.length) {
            setSelectedRuleId(null);
            return;
        }
        if (!rules.some((rule) => rule.id === selectedRuleId)) {
            setSelectedRuleId(rules[0].id);
        }
    }, [rules, selectedRuleId]);

    const selectedRule = useMemo(
        () => rules.find((rule) => rule.id === selectedRuleId) || null,
        [rules, selectedRuleId]
    );

    function update(
        ruleId: string,
        updater: (rule: ContextAutomationRule) => ContextAutomationRule
    ) {
        onRulesChange(
            updateRule(rules, ruleId, (rule) =>
                normalizeContextRule(updater(rule))
            )
        );
    }

    function addRule() {
        const nextRule = createContextRule(t(`${I18N_ROOT}.room_rule_default`));
        setSelectedRuleId(nextRule.id);
        onRulesChange([...rules, nextRule]);
    }

    function removeRule(ruleId: string) {
        const ruleIndex = rules.findIndex((rule) => rule.id === ruleId);
        const nextRules = rules.filter((rule) => rule.id !== ruleId);
        if (selectedRuleId === ruleId) {
            setSelectedRuleId(
                nextRules[Math.min(ruleIndex, nextRules.length - 1)]?.id ?? null
            );
        }
        onRulesChange(nextRules);
    }

    const customRulesList = (
        <RuleList
            title={t(`${I18N_ROOT}.room_social_rules`)}
            description={t(`${I18N_ROOT}.room_social_rules_description`)}
            addLabel={t(`${I18N_ROOT}.add_rule`)}
            disabled={loading}
            isEmpty={!rules.length}
            emptyTitle={t(`${I18N_ROOT}.no_custom_room_rules`)}
            emptyDescription={t(`${I18N_ROOT}.room_social_rules_description`)}
            onAdd={addRule}
        >
            {rules.map((rule) => (
                <RuleListItem
                    key={rule.id}
                    selected={rule.id === selectedRuleId}
                    title={ruleTitle(rule, t)}
                    description={t(contextPresetLabelKeyFromValue(rule.preset))}
                    enabled={rule.enabled !== false}
                    disabled={loading}
                    removeLabel={t(`${I18N_ROOT}.remove_room_rule`)}
                    badges={
                        <>
                            <RuleSummaryBadge>
                                {t(priorityLabelKeyFromNumber(rule.priority))}
                            </RuleSummaryBadge>
                            <RuleSummaryBadge>
                                {actionSummary(rule, t)}
                            </RuleSummaryBadge>
                        </>
                    }
                    onSelect={() => setSelectedRuleId(rule.id)}
                    onEnabledChange={(checked) =>
                        update(rule.id, (current) => ({
                            ...current,
                            enabled: checked
                        }))
                    }
                    onRemove={() => removeRule(rule.id)}
                />
            ))}
        </RuleList>
    );

    const customRulesEditor = (
        <RuleEditorPanel
            title={
                selectedRule
                    ? ruleTitle(selectedRule, t)
                    : t(`${I18N_ROOT}.room_rule_default`)
            }
            description={
                selectedRule
                    ? t(contextPresetLabelKeyFromValue(selectedRule.preset))
                    : t(`${I18N_ROOT}.no_custom_room_rules`)
            }
        >
            {selectedRule ? (
                <FieldGroup>
                    <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_10rem]">
                        <Field>
                            <FieldLabel>
                                {t(`${I18N_ROOT}.rule_name`)}
                            </FieldLabel>
                            <Input
                                value={selectedRule.label || ''}
                                disabled={loading}
                                onChange={(event) =>
                                    update(selectedRule.id, (current) => ({
                                        ...current,
                                        label: event.target.value
                                    }))
                                }
                            />
                        </Field>
                        <Field>
                            <FieldLabel>
                                {t(`${I18N_ROOT}.priority`)}
                            </FieldLabel>
                            <Select
                                value={priorityValueFromNumber(
                                    selectedRule.priority
                                )}
                                disabled={loading}
                                items={priorityOptions.map((option) => ({
                                    value: option.value,
                                    label: t(option.labelKey)
                                }))}
                                onValueChange={(value) =>
                                    update(selectedRule.id, (current) => ({
                                        ...current,
                                        priority: priorityNumberFromValue(value)
                                    }))
                                }
                            >
                                <SelectTrigger>
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectGroup>
                                        {priorityOptions.map((option) => (
                                            <SelectItem
                                                key={option.value}
                                                value={option.value}
                                            >
                                                {t(option.labelKey)}
                                            </SelectItem>
                                        ))}
                                    </SelectGroup>
                                </SelectContent>
                            </Select>
                        </Field>
                    </div>
                    <FieldSet className="border-t pt-4">
                        <FieldLegend variant="label">
                            {t(`${I18N_ROOT}.when`)}
                        </FieldLegend>
                        <FieldGroup>
                            <Field>
                                <Select
                                    value={selectedRule.preset || 'alone'}
                                    disabled={loading}
                                    items={contextPresetOptions.map(
                                        (preset) => ({
                                            value: preset.value,
                                            label: t(preset.labelKey)
                                        })
                                    )}
                                    onValueChange={(value) =>
                                        update(selectedRule.id, (current) => ({
                                            ...current,
                                            preset: value ?? ''
                                        }))
                                    }
                                >
                                    <SelectTrigger
                                        aria-label={t(`${I18N_ROOT}.when`)}
                                    >
                                        <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectGroup>
                                            {contextPresetOptions.map(
                                                (preset) => (
                                                    <SelectItem
                                                        key={preset.value}
                                                        value={preset.value}
                                                    >
                                                        {t(preset.labelKey)}
                                                    </SelectItem>
                                                )
                                            )}
                                        </SelectGroup>
                                    </SelectContent>
                                </Select>
                            </Field>
                            {selectedRule.preset === 'withSelectedGroups' ? (
                                <Field>
                                    <FieldLabel>
                                        {t(`${I18N_ROOT}.friend_groups`)}
                                    </FieldLabel>
                                    <CompactCheckList
                                        values={
                                            selectedRule.selectedGroups || []
                                        }
                                        options={groupOptions}
                                        disabled={loading}
                                        onChange={(next) =>
                                            update(
                                                selectedRule.id,
                                                (current) => ({
                                                    ...current,
                                                    selectedGroups: next
                                                })
                                            )
                                        }
                                    />
                                </Field>
                            ) : null}
                            {selectedRule.preset === 'inFavoriteWorlds' ? (
                                <Field>
                                    <FieldLabel>
                                        {t(`${I18N_ROOT}.world_groups`)}
                                    </FieldLabel>
                                    <FieldDescription>
                                        {t(`${I18N_ROOT}.world_groups_hint`)}
                                    </FieldDescription>
                                    <CompactCheckList
                                        values={
                                            selectedRule.selectedWorldGroups ||
                                            []
                                        }
                                        options={worldGroupOptions}
                                        disabled={loading}
                                        onChange={(next) =>
                                            update(
                                                selectedRule.id,
                                                (current) => ({
                                                    ...current,
                                                    selectedWorldGroups: next
                                                })
                                            )
                                        }
                                    />
                                </Field>
                            ) : null}
                            {selectedRule.preset === 'friendCountAtLeast' ? (
                                <Field>
                                    <FieldLabel>
                                        {t(`${I18N_ROOT}.minimum_friends`)}
                                    </FieldLabel>
                                    <Input
                                        type="number"
                                        min="1"
                                        value={
                                            selectedRule.friendCountValue || 1
                                        }
                                        disabled={loading}
                                        onChange={(event) =>
                                            update(
                                                selectedRule.id,
                                                (current) => ({
                                                    ...current,
                                                    friendCountValue:
                                                        Number.parseInt(
                                                            event.target.value,
                                                            10
                                                        ) || 1
                                                })
                                            )
                                        }
                                    />
                                </Field>
                            ) : null}
                            {selectedRule.preset === 'playerCountAtLeast' ? (
                                <Field>
                                    <FieldLabel>
                                        {t(`${I18N_ROOT}.minimum_players`)}
                                    </FieldLabel>
                                    <Input
                                        type="number"
                                        min="1"
                                        value={
                                            selectedRule.playerCountValue || 1
                                        }
                                        disabled={loading}
                                        onChange={(event) =>
                                            update(
                                                selectedRule.id,
                                                (current) => ({
                                                    ...current,
                                                    playerCountValue:
                                                        Number.parseInt(
                                                            event.target.value,
                                                            10
                                                        ) || 1
                                                })
                                            )
                                        }
                                    />
                                </Field>
                            ) : null}
                            {selectedRule.preset === 'withSelectedFriend' ? (
                                <Field>
                                    <FieldLabel>
                                        {t(`${I18N_ROOT}.friend_user_ids`)}
                                    </FieldLabel>
                                    <Input
                                        value={(
                                            selectedRule.specificFriendIds || []
                                        ).join(', ')}
                                        disabled={loading}
                                        placeholder="usr_..., usr_..."
                                        onChange={(event) =>
                                            update(
                                                selectedRule.id,
                                                (current) => ({
                                                    ...current,
                                                    specificFriendIds:
                                                        parseUserIds(
                                                            event.target.value
                                                        )
                                                })
                                            )
                                        }
                                    />
                                </Field>
                            ) : null}
                            <Field>
                                <FieldLabel>
                                    {t(`${I18N_ROOT}.room_types`)}
                                </FieldLabel>
                                <FieldDescription>
                                    {t(`${I18N_ROOT}.room_types_hint`)}
                                </FieldDescription>
                                <CompactCheckList
                                    values={
                                        selectedRule.selectedInstanceTypes || []
                                    }
                                    options={instanceOptions}
                                    disabled={loading}
                                    onChange={(next) =>
                                        update(selectedRule.id, (current) => ({
                                            ...current,
                                            selectedInstanceTypes: next
                                        }))
                                    }
                                />
                            </Field>
                        </FieldGroup>
                    </FieldSet>
                    <PresenceRuleActionFields
                        idPrefix={selectedRule.id}
                        disabled={loading}
                        status={selectedRule.actions?.status || 'no-change'}
                        statusDescriptionEnabled={hasAction(
                            selectedRule,
                            'statusDescription'
                        )}
                        statusDescription={
                            selectedRule.actions?.statusDescription || ''
                        }
                        onStatusChange={(value) =>
                            update(selectedRule.id, (current) =>
                                value === 'no-change'
                                    ? removeAction(current, 'status')
                                    : updateAction(current, { status: value })
                            )
                        }
                        onStatusDescriptionEnabledChange={(checked) =>
                            update(selectedRule.id, (current) =>
                                checked
                                    ? updateAction(current, {
                                          statusDescription: ''
                                      })
                                    : removeAction(current, 'statusDescription')
                            )
                        }
                        onStatusDescriptionChange={(value) =>
                            update(selectedRule.id, (current) =>
                                updateAction(current, {
                                    statusDescription: value
                                })
                            )
                        }
                    />
                </FieldGroup>
            ) : (
                <Empty className="min-h-[18rem] border">
                    <EmptyHeader>
                        <EmptyTitle>
                            {t(`${I18N_ROOT}.no_custom_room_rules`)}
                        </EmptyTitle>
                        <EmptyDescription>
                            {t(`${I18N_ROOT}.room_social_rules_description`)}
                        </EmptyDescription>
                    </EmptyHeader>
                </Empty>
            )}
        </RuleEditorPanel>
    );

    return (
        <AutomationSplitLayout
            list={customRulesList}
            editor={customRulesEditor}
        />
    );
}
