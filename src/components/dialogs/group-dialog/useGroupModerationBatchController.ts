import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import type { EntityRecord } from '@/domain/entities/profileEntities';
import { userFacingErrorMessage } from '@/lib/errorDisplay';
import {
    commands,
    type GroupModerationBatchAction
} from '@/platform/tauri/bindings';
import { useModalStore } from '@/state/modalStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import { buildGroupModerationBatchInput } from './groupModerationBatch';
import type { GroupModerationBulkProgress } from './GroupModerationBulkPanel';
import { moderationRowLabel, moderationRowUserId } from './groupModerationRows';
import { resolveGroupModerationBatchProgress } from './groupModerationWorkspaceContext';

export function useGroupModerationBatchController({
    activeTab,
    endpoint,
    groupId,
    rows,
    resetKey,
    reload
}: {
    activeTab: string;
    endpoint: string;
    groupId: string;
    rows: EntityRecord[];
    resetKey: string;
    reload: () => void;
}) {
    const { t } = useTranslation();
    const confirm = useModalStore((state) => state.confirm);
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const currentAuthEndpoint = useRuntimeStore(
        (state) => state.auth.currentUserEndpoint
    );
    const batchProgressEvent = useRuntimeStore(
        (state) => state.runtimeEvents.groupModerationBatchProgress
    );
    const [selectedByTab, setSelectedByTab] = useState<
        Record<string, Set<string>>
    >({});
    const [bulkBusy, setBulkBusy] = useState(false);
    const [bulkProgress, setBulkProgress] =
        useState<GroupModerationBulkProgress | null>(null);
    const bulkProgressEventCountRef = useRef(0);
    const bulkRunSequenceRef = useRef(0);
    const selectedIds = selectedByTab[activeTab] || null;
    const selectedRows = selectedIds
        ? rows.filter((row) => selectedIds.has(moderationRowUserId(row)))
        : [];

    useEffect(() => {
        bulkRunSequenceRef.current += 1;
        setSelectedByTab({});
        setBulkBusy(false);
        setBulkProgress(null);
    }, [resetKey]);

    useEffect(() => {
        const nextProgress = resolveGroupModerationBatchProgress({
            busy: bulkBusy,
            currentAuthEndpoint,
            currentUserId,
            endpoint,
            event: batchProgressEvent,
            groupId,
            previousEventCount: bulkProgressEventCountRef.current
        });
        if (nextProgress) {
            setBulkProgress(nextProgress);
        }
    }, [
        batchProgressEvent,
        bulkBusy,
        currentAuthEndpoint,
        currentUserId,
        endpoint,
        groupId
    ]);

    function toggleSelectedVisible(userIds: string[], checked: boolean) {
        setSelectedByTab((current) => {
            const next = new Set(current[activeTab] || []);
            for (const userId of userIds) {
                if (checked) {
                    next.add(userId);
                } else {
                    next.delete(userId);
                }
            }
            return { ...current, [activeTab]: next };
        });
    }

    function toggleSelectedRow(userId: string, checked: boolean) {
        if (!userId) {
            return;
        }
        toggleSelectedVisible([userId], checked);
    }

    function clearSelection() {
        setSelectedByTab((current) => ({
            ...current,
            [activeTab]: new Set()
        }));
    }

    async function runBulkAction({
        action,
        label,
        destructive = false,
        roleIds
    }: {
        action: GroupModerationBatchAction;
        label: string;
        destructive?: boolean;
        roleIds?: string[];
    }) {
        if (bulkBusy || !selectedRows.length) {
            return;
        }
        const targetRows = selectedRows;
        const result = await confirm({
            title: t('dialog.group.dynamic.value_group_user', { value: label }),
            description: t(
                'dialog.group_member_moderation.bulk_action_confirm',
                { count: targetRows.length }
            ),
            confirmText: label,
            cancelText: t('common.actions.cancel'),
            destructive
        });
        if (!result.ok) {
            return;
        }
        const batchOwnerUserId = useRuntimeStore.getState().auth.currentUserId;
        const batchEndpoint = endpoint;
        if (
            !batchOwnerUserId ||
            batchOwnerUserId !== currentUserId ||
            useRuntimeStore.getState().auth.currentUserEndpoint !==
                batchEndpoint
        ) {
            return;
        }

        const batchRunSequence = bulkRunSequenceRef.current + 1;
        bulkRunSequenceRef.current = batchRunSequence;
        const isCurrentBatchRun = () => {
            const auth = useRuntimeStore.getState().auth;
            return (
                bulkRunSequenceRef.current === batchRunSequence &&
                auth.currentUserId === batchOwnerUserId &&
                auth.currentUserEndpoint === batchEndpoint
            );
        };
        setBulkBusy(true);
        bulkProgressEventCountRef.current = batchProgressEvent?.count ?? 0;
        setBulkProgress({ current: 0, total: targetRows.length });
        try {
            const batchResult = await commands.appGroupModerationBatch(
                buildGroupModerationBatchInput({
                    action,
                    expectedEndpoint: endpoint,
                    expectedOwnerUserId: batchOwnerUserId,
                    groupId,
                    roleIds,
                    rows: targetRows
                })
            );
            if (
                !isCurrentBatchRun() ||
                batchResult.ownerUserId !== batchOwnerUserId ||
                batchResult.endpoint !== batchEndpoint
            ) {
                return;
            }
            setBulkProgress({
                current: batchResult.total,
                total: batchResult.total
            });
            const rowsByUserId = new Map(
                targetRows.map((row) => [moderationRowUserId(row), row])
            );
            for (const item of batchResult.items) {
                if (
                    item.state !== 'failed' &&
                    item.state !== 'partiallyApplied' &&
                    item.state !== 'notAttempted'
                ) {
                    continue;
                }
                const row = rowsByUserId.get(item.userId);
                toast.error(
                    `${moderationRowLabel(row || item.userId)}: ${userFacingErrorMessage(
                        item.message,
                        t('dialog.group.toast.value_failed', { value: label })
                    )}`
                );
            }
            if (batchResult.succeeded) {
                toast.success(
                    t('dialog.group_member_moderation.bulk_action_completed', {
                        count: batchResult.succeeded,
                        value: label
                    })
                );
            }
        } catch (actionError) {
            if (isCurrentBatchRun()) {
                toast.error(
                    userFacingErrorMessage(
                        actionError,
                        t('dialog.group.toast.value_failed', { value: label })
                    )
                );
            }
        } finally {
            if (bulkRunSequenceRef.current === batchRunSequence) {
                setBulkBusy(false);
                setBulkProgress(null);
                if (isCurrentBatchRun()) {
                    clearSelection();
                    reload();
                }
            }
        }
    }

    return {
        bulkBusy,
        bulkProgress,
        clearSelection,
        runBulkAddRoles: (roleIds: string[]) =>
            runBulkAction({
                action: { type: 'addRoles' },
                label: t('dialog.group_member_moderation.add_roles'),
                roleIds
            }),
        runBulkBan: () =>
            runBulkAction({
                action: { type: 'ban' },
                label: t('dialog.group_member_moderation.ban'),
                destructive: true
            }),
        runBulkKick: () =>
            runBulkAction({
                action: { type: 'kick' },
                label: t('dialog.group_member_moderation.kick'),
                destructive: true
            }),
        runBulkRemoveRoles: (roleIds: string[]) =>
            runBulkAction({
                action: { type: 'removeRoles' },
                label: t('dialog.group_member_moderation.remove_roles'),
                roleIds
            }),
        runBulkSaveNote: (note: string) =>
            runBulkAction({
                action: { type: 'saveNote', note },
                label: t('dialog.group_member_moderation.save_note')
            }),
        runBulkUnban: () =>
            runBulkAction({
                action: { type: 'unban' },
                label: t('dialog.group_member_moderation.unban')
            }),
        selectedIds,
        selectedRows,
        toggleSelectedRow,
        toggleSelectedVisible
    };
}
