import { useEffect, useState } from 'react';

import type { EntityRecord } from '@/domain/entities/profileEntities';
import groupProfileRepository from '@/repositories/groupProfileRepository';

import type { GroupModerationTabValue } from './groupDialogUtils';
import { moderationRowUserId } from './groupModerationRows';

export function isGroupModerationEntityRecord(
    value: unknown
): value is EntityRecord {
    return Boolean(value && typeof value === 'object');
}

export function useGroupModerationTabData({
    activeTab,
    endpoint,
    groupId,
    reloadToken,
    resetKey
}: {
    activeTab: GroupModerationTabValue | '';
    endpoint: string;
    groupId: string;
    reloadToken: number;
    resetKey: string;
}) {
    const [rowsByTab, setRowsByTab] = useState<Record<string, EntityRecord[]>>(
        {}
    );
    const [statusByTab, setStatusByTab] = useState<Record<string, string>>({});
    const [errorsByTab, setErrorsByTab] = useState<Record<string, string>>({});

    useEffect(() => {
        setRowsByTab({});
        setStatusByTab({});
        setErrorsByTab({});
    }, [resetKey]);

    useEffect(() => {
        if (!activeTab || activeTab === 'logs' || activeTab === 'members') {
            return;
        }

        let active = true;
        setStatusByTab((current) => ({
            ...current,
            [activeTab]: 'running'
        }));
        setErrorsByTab((current) => ({ ...current, [activeTab]: '' }));

        const request =
            activeTab === 'bans'
                ? groupProfileRepository.getAllGroupBans({ groupId })
                : activeTab === 'invites'
                  ? groupProfileRepository.getAllGroupInvites({ groupId })
                  : activeTab === 'requests'
                    ? groupProfileRepository.getAllGroupJoinRequests({
                          groupId,
                          blocked: false
                      })
                    : groupProfileRepository.getAllGroupJoinRequests({
                          groupId,
                          blocked: true
                      });

        request
            .then((nextRows) => {
                if (!active) {
                    return;
                }
                setRowsByTab((current) => ({
                    ...current,
                    [activeTab]: Array.isArray(nextRows)
                        ? nextRows.filter(isGroupModerationEntityRecord)
                        : []
                }));
                setStatusByTab((current) => ({
                    ...current,
                    [activeTab]: 'ready'
                }));
            })
            .catch((requestError: unknown) => {
                if (!active) {
                    return;
                }
                setStatusByTab((current) => ({
                    ...current,
                    [activeTab]: 'error'
                }));
                setErrorsByTab((current) => ({
                    ...current,
                    [activeTab]:
                        requestError instanceof Error
                            ? requestError.message
                            : 'Failed to load moderation data.'
                }));
            });

        return () => {
            active = false;
        };
    }, [activeTab, endpoint, groupId, reloadToken]);

    function removeRow(userId: string) {
        setRowsByTab((current) => ({
            ...current,
            [activeTab]: (current[activeTab] || []).filter(
                (item) => moderationRowUserId(item) !== userId
            )
        }));
        setStatusByTab((current) => ({
            ...current,
            [activeTab]: 'ready'
        }));
        setErrorsByTab((current) => ({ ...current, [activeTab]: '' }));
    }

    return {
        rows: rowsByTab[activeTab] || [],
        status: statusByTab[activeTab] || '',
        error: errorsByTab[activeTab] || '',
        removeRow
    };
}
