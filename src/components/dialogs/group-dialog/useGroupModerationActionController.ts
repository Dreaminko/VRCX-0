import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import type { EntityRecord } from '@/domain/entities/profileEntities';
import groupProfileRepository from '@/repositories/groupProfileRepository';
import { useModalStore } from '@/state/modalStore';

import {
    moderationRowLabel,
    moderationRowUserId,
    type GroupModerationAction
} from './groupModerationRows';

export function useGroupModerationActionController({
    activeTab,
    groupId,
    resetKey,
    removeMemberRow,
    removeTabRow
}: {
    activeTab: string;
    groupId: string;
    resetKey: string;
    removeMemberRow: (userId: string) => void;
    removeTabRow: (userId: string) => void;
}) {
    const { t } = useTranslation();
    const confirm = useModalStore((state) => state.confirm);
    const [actionKey, setActionKey] = useState('');

    useEffect(() => {
        setActionKey('');
    }, [resetKey]);

    async function runAction(action: GroupModerationAction, row: EntityRecord) {
        const userId = moderationRowUserId(row);
        if (!userId || actionKey) {
            return;
        }
        const label = moderationRowLabel(row);
        const result = await confirm({
            title: t('dialog.group.dynamic.value_group_user', {
                value: action.label
            }),
            description: label,
            confirmText: action.label,
            cancelText: t('common.actions.cancel'),
            destructive: Boolean(action.destructive)
        });
        if (!result.ok) {
            return;
        }

        setActionKey(`${activeTab}:${action.key}:${userId}`);
        try {
            if (action.key === 'kick') {
                await groupProfileRepository.kickGroupMember({
                    groupId,
                    userId
                });
            } else if (action.key === 'ban') {
                await groupProfileRepository.banGroupMember({
                    groupId,
                    userId
                });
            } else if (action.key === 'unban') {
                await groupProfileRepository.unbanGroupMember({
                    groupId,
                    userId
                });
            } else if (action.key === 'delete-invite') {
                await groupProfileRepository.deleteSentGroupInvite({
                    groupId,
                    userId
                });
            } else if (action.key === 'accept-request') {
                await groupProfileRepository.respondGroupJoinRequest({
                    groupId,
                    userId,
                    action: 'accept'
                });
            } else if (action.key === 'reject-request') {
                await groupProfileRepository.respondGroupJoinRequest({
                    groupId,
                    userId,
                    action: 'reject'
                });
            } else if (action.key === 'block-request') {
                await groupProfileRepository.respondGroupJoinRequest({
                    groupId,
                    userId,
                    action: 'reject',
                    block: true
                });
            } else if (action.key === 'delete-blocked') {
                await groupProfileRepository.deleteBlockedGroupRequest({
                    groupId,
                    userId
                });
            }
            if (activeTab === 'members') {
                removeMemberRow(userId);
            } else {
                removeTabRow(userId);
            }
            toast.success(
                t('dialog.group.dynamic.value_completed', {
                    value: action.label
                })
            );
        } catch (actionError) {
            toast.error(
                actionError instanceof Error
                    ? actionError.message
                    : t('dialog.group.toast.value_failed', {
                          value: action.label
                      })
            );
        } finally {
            setActionKey('');
        }
    }

    return { actionKey, runAction };
}
