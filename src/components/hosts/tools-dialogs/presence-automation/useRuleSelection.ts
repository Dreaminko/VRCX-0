import { useEffect, useMemo, useState } from 'react';

export function useRuleSelection<TRule extends { id: string }>(
    rules: readonly TRule[]
) {
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

    function removeRule(ruleId: string): TRule[] {
        const ruleIndex = rules.findIndex((rule) => rule.id === ruleId);
        const nextRules = rules.filter((rule) => rule.id !== ruleId);
        if (selectedRuleId === ruleId) {
            setSelectedRuleId(
                nextRules[Math.min(ruleIndex, nextRules.length - 1)]?.id ?? null
            );
        }
        return nextRules;
    }

    return { selectedRule, selectedRuleId, setSelectedRuleId, removeRule };
}
