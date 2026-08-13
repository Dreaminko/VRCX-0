import type { TFunction } from 'i18next';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { formatDateTime } from '@/lib/dateTime';
import { commands } from '@/platform/tauri/bindings';
import configRepository from '@/repositories/configRepository';
import { getCurrentAppLauncherSnapshot } from '@/services/appLauncherSnapshotService';
import { getProfileBackupSettings } from '@/services/profileBackupService';
import { TOOLS_STATUS_UPDATED_EVENT } from '@/shared/constants/tools';
import { useProfileBackupStore } from '@/state/profileBackupStore';

export type ToolStatusSummary = {
    label: string;
    tone: 'active' | 'neutral';
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

export function countPresenceRules(rules: readonly unknown[] | null): {
    enabled: number;
    total: number;
} {
    const configuredRules = (rules ?? []).filter(isRecord);
    return {
        enabled: configuredRules.filter((rule) => rule.enabled !== false)
            .length,
        total: configuredRules.length
    };
}

async function loadToolStatusSummaries(
    t: TFunction
): Promise<Map<string, ToolStatusSummary>> {
    const [
        timeRules,
        contextRules,
        inviteMode,
        endpoints,
        appLauncher,
        backupSettings
    ] = await Promise.all([
        commands.appPresenceAutomationRulesGet('time').catch(() => null),
        commands.appPresenceAutomationRulesGet('context').catch(() => null),
        configRepository
            .getString('autoAcceptInviteRequests', 'Off')
            .catch(() => null),
        commands.appLlmEndpointList().catch(() => null),
        getCurrentAppLauncherSnapshot().catch(() => null),
        getProfileBackupSettings().catch(() => null)
    ]);

    const next = new Map<string, ToolStatusSummary>();
    for (const [toolKey, rules] of [
        ['presence-schedule', timeRules],
        ['presence-room-rules', contextRules]
    ] as const) {
        const counts = countPresenceRules(rules);
        if (counts.enabled > 0) {
            next.set(toolKey, {
                label:
                    counts.enabled === counts.total
                        ? t('view.tools.status.rules_enabled', {
                              count: counts.enabled
                          })
                        : t('view.tools.status.rules_enabled_of_total', {
                              enabled: counts.enabled,
                              total: counts.total
                          }),
                tone: 'active'
            });
        } else if (counts.total > 0) {
            next.set(toolKey, {
                label: t('view.tools.status.rules_configured_off', {
                    count: counts.total
                }),
                tone: 'neutral'
            });
        }
    }

    if (inviteMode && inviteMode !== 'Off') {
        next.set('presence-invite-requests', {
            label: t('view.tools.status.auto_reply_enabled'),
            tone: 'active'
        });
    }

    if (appLauncher?.entries.length) {
        next.set('app-launcher', {
            label: t(
                appLauncher.enabled
                    ? 'view.tools.status.apps_enabled'
                    : 'view.tools.status.apps_configured_off',
                { count: appLauncher.entries.length }
            ),
            tone: appLauncher.enabled ? 'active' : 'neutral'
        });
    }

    if (backupSettings?.lastAutoAt) {
        next.set('profile-backup', {
            label: t('view.tools.status.last_backup', {
                date: formatDateTime(backupSettings.lastAutoAt, {
                    dateStyle: 'medium',
                    timeStyle: 'short'
                })
            }),
            tone: 'active'
        });
    } else if (backupSettings?.autoEnabled) {
        next.set('profile-backup', {
            label: t('view.tools.status.automatic_backup_enabled'),
            tone: 'active'
        });
    } else if (backupSettings?.autoTargetDir) {
        next.set('profile-backup', {
            label: t('view.tools.status.automatic_backup_configured_off'),
            tone: 'neutral'
        });
    }

    if (endpoints?.length) {
        next.set('llm-endpoints', {
            label: t('view.tools.status.connections_configured', {
                count: endpoints.length
            }),
            tone: 'neutral'
        });
    }

    return next;
}

export function useToolStatusSummaries(): Map<string, ToolStatusSummary> {
    const { t } = useTranslation();
    const backupOutcomeRevision = useProfileBackupStore(
        (state) => state.status.lastOutcome?.revision ?? -1
    );
    const [statusByToolKey, setStatusByToolKey] = useState(
        () => new Map<string, ToolStatusSummary>()
    );

    useEffect(() => {
        let active = true;
        let requestRevision = 0;
        const refresh = () => {
            const expectedRevision = ++requestRevision;
            void loadToolStatusSummaries(t).then((next) => {
                if (active && expectedRevision === requestRevision) {
                    setStatusByToolKey(next);
                }
            });
        };
        refresh();
        window.addEventListener(TOOLS_STATUS_UPDATED_EVENT, refresh);
        return () => {
            active = false;
            window.removeEventListener(TOOLS_STATUS_UPDATED_EVENT, refresh);
        };
    }, [backupOutcomeRevision, t]);

    return statusByToolKey;
}
