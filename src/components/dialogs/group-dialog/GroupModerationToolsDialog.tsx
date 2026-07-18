import { UploadIcon } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import type {
    EntityRecord,
    GroupProfileRecord
} from '@/domain/entities/profileEntities';
import { userFacingErrorMessage } from '@/lib/errorDisplay';
import groupProfileRepository from '@/repositories/groupProfileRepository';
import { useModalStore } from '@/state/modalStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { Button } from '@/ui/shadcn/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle
} from '@/ui/shadcn/dialog';
import { Tabs, TabsList, TabsTrigger } from '@/ui/shadcn/tabs';

import { hasGroupPermission } from './groupDialogUtils';
import { GroupModerationBanImportDialog } from './GroupModerationBanImportDialog';
import {
    GroupModerationBulkPanel,
    type GroupModerationBulkProgress
} from './GroupModerationBulkPanel';
import { GroupModerationLogsPanel } from './GroupModerationLogsPanel';
import {
    getGroupModerationTabs,
    moderationRowLabel,
    moderationRowRoleIds,
    moderationRowUserId,
    resolveGroupModerationActiveTab,
    type GroupModerationAction
} from './groupModerationRows';
import { GroupModerationTabPanel } from './GroupModerationTabPanel';

function isEntityRecord(value: unknown): value is EntityRecord {
    return Boolean(value && typeof value === 'object');
}

const BULK_SELECTABLE_TABS = new Set(['bans', 'members']);

export function GroupModerationToolsDialog({
    open,
    onOpenChange,
    group,
    endpoint
}: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    group: GroupProfileRecord;
    endpoint: string;
}) {
    const { t } = useTranslation();
    const confirm = useModalStore((state) => state.confirm);
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const [activeTab, setActiveTab] = useState('members');
    const [rowsByTab, setRowsByTab] = useState<Record<string, EntityRecord[]>>(
        {}
    );
    const [statusByTab, setStatusByTab] = useState<Record<string, string>>({});
    const [errorsByTab, setErrorsByTab] = useState<Record<string, string>>({});
    const [search, setSearch] = useState('');
    const [pageSize, setPageSize] = useState(25);
    const [pageIndex, setPageIndex] = useState(0);
    const [reloadToken, setReloadToken] = useState(0);
    const [actionKey, setActionKey] = useState('');
    const [selectedByTab, setSelectedByTab] = useState<
        Record<string, Set<string>>
    >({});
    const [bulkBusy, setBulkBusy] = useState(false);
    const [bulkProgress, setBulkProgress] =
        useState<GroupModerationBulkProgress | null>(null);
    const [banImportOpen, setBanImportOpen] = useState(false);
    const resetKeyRef = useRef('');
    const moderationTabs = useMemo(
        () => getGroupModerationTabs(t, group),
        [group.id, group.myMember, group.roles, t]
    );
    const resetKey = `${endpoint}\u0000${group.id || ''}`;
    const rows = rowsByTab[activeTab] || [];
    const loading = statusByTab[activeTab] === 'running';
    const error = errorsByTab[activeTab] || '';
    const selectedIds = selectedByTab[activeTab] || null;
    const selectedRows = selectedIds
        ? rows.filter((row) => selectedIds.has(moderationRowUserId(row)))
        : [];
    const bulkSelectable = BULK_SELECTABLE_TABS.has(activeTab);

    useEffect(() => {
        if (!open) {
            resetKeyRef.current = '';
            return;
        }

        if (resetKeyRef.current !== resetKey) {
            resetKeyRef.current = resetKey;
            setActiveTab(
                resolveGroupModerationActiveTab('members', moderationTabs)
            );
            setRowsByTab({});
            setStatusByTab({});
            setErrorsByTab({});
            setSearch('');
            setPageIndex(0);
            setActionKey('');
            setSelectedByTab({});
            setBulkBusy(false);
            setBulkProgress(null);
            setBanImportOpen(false);
            return;
        }

        setActiveTab((current) =>
            resolveGroupModerationActiveTab(current, moderationTabs)
        );
    }, [moderationTabs, open, resetKey]);

    useEffect(() => {
        setSearch('');
        setPageIndex(0);
    }, [activeTab]);

    useEffect(() => {
        if (!open || !activeTab || activeTab === 'logs') {
            return;
        }

        let active = true;
        setStatusByTab((current) => ({
            ...current,
            [activeTab]: 'running'
        }));
        setErrorsByTab((current) => ({ ...current, [activeTab]: '' }));

        const request =
            activeTab === 'members'
                ? groupProfileRepository.getAllGroupMembers({
                      groupId: group.id,
                      endpoint
                  })
                : activeTab === 'bans'
                  ? groupProfileRepository.getAllGroupBans({
                        groupId: group.id,
                        endpoint
                    })
                  : activeTab === 'invites'
                    ? groupProfileRepository.getAllGroupInvites({
                          groupId: group.id,
                          endpoint
                      })
                    : activeTab === 'requests'
                      ? groupProfileRepository.getAllGroupJoinRequests({
                            groupId: group.id,
                            endpoint,
                            blocked: false
                        })
                      : groupProfileRepository.getAllGroupJoinRequests({
                            groupId: group.id,
                            endpoint,
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
                        ? nextRows.filter(isEntityRecord)
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
    }, [activeTab, endpoint, group.id, open, reloadToken]);

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
        setSelectedByTab((current) => ({ ...current, [activeTab]: new Set() }));
    }

    async function runBulkAction({
        label,
        destructive = false,
        skipSelf,
        action
    }: {
        label: string;
        destructive?: boolean;
        skipSelf: boolean;
        action: (row: EntityRecord) => Promise<void>;
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

        setBulkBusy(true);
        setBulkProgress({ current: 0, total: targetRows.length });
        let successCount = 0;
        for (let index = 0; index < targetRows.length; index += 1) {
            const row = targetRows[index];
            setBulkProgress({ current: index + 1, total: targetRows.length });
            const userId = moderationRowUserId(row);
            if (skipSelf && currentUserId && userId === currentUserId) {
                continue;
            }
            try {
                await action(row);
                successCount += 1;
            } catch (actionError) {
                toast.error(
                    `${moderationRowLabel(row)}: ${userFacingErrorMessage(
                        actionError,
                        t('dialog.group.toast.value_failed', { value: label })
                    )}`
                );
            }
        }

        setBulkBusy(false);
        setBulkProgress(null);
        clearSelection();
        setReloadToken((value) => value + 1);
        if (successCount) {
            toast.success(
                t('dialog.group_member_moderation.bulk_action_completed', {
                    count: successCount,
                    value: label
                })
            );
        }
    }

    function runBulkKick() {
        return runBulkAction({
            label: t('dialog.group_member_moderation.kick'),
            destructive: true,
            skipSelf: true,
            action: async (row) => {
                await groupProfileRepository.kickGroupMember({
                    groupId: group.id,
                    userId: moderationRowUserId(row),
                    endpoint
                });
            }
        });
    }

    function runBulkBan() {
        return runBulkAction({
            label: t('dialog.group_member_moderation.ban'),
            destructive: true,
            skipSelf: true,
            action: async (row) => {
                await groupProfileRepository.banGroupMember({
                    groupId: group.id,
                    userId: moderationRowUserId(row),
                    endpoint
                });
            }
        });
    }

    function runBulkUnban() {
        return runBulkAction({
            label: t('dialog.group_member_moderation.unban'),
            skipSelf: true,
            action: async (row) => {
                await groupProfileRepository.unbanGroupMember({
                    groupId: group.id,
                    userId: moderationRowUserId(row),
                    endpoint
                });
            }
        });
    }

    function runBulkSaveNote(note: string) {
        return runBulkAction({
            label: t('dialog.group_member_moderation.save_note'),
            skipSelf: false,
            action: async (row) => {
                await groupProfileRepository.setGroupMemberProps({
                    groupId: group.id,
                    userId: moderationRowUserId(row),
                    params: { managerNotes: note },
                    endpoint
                });
            }
        });
    }

    function runBulkAddRoles(roleIds: string[]) {
        return runBulkAction({
            label: t('dialog.group_member_moderation.add_roles'),
            skipSelf: true,
            action: async (row) => {
                const userId = moderationRowUserId(row);
                const currentRoleIds = new Set(moderationRowRoleIds(row));
                for (const roleId of roleIds) {
                    if (currentRoleIds.has(roleId)) {
                        continue;
                    }
                    await groupProfileRepository.addGroupMemberRole({
                        groupId: group.id,
                        userId,
                        roleId,
                        endpoint
                    });
                }
            }
        });
    }

    function runBulkRemoveRoles(roleIds: string[]) {
        return runBulkAction({
            label: t('dialog.group_member_moderation.remove_roles'),
            skipSelf: true,
            action: async (row) => {
                const userId = moderationRowUserId(row);
                const currentRoleIds = new Set(moderationRowRoleIds(row));
                for (const roleId of roleIds) {
                    if (!currentRoleIds.has(roleId)) {
                        continue;
                    }
                    await groupProfileRepository.removeGroupMemberRole({
                        groupId: group.id,
                        userId,
                        roleId,
                        endpoint
                    });
                }
            }
        });
    }

    async function runModerationAction(
        action: GroupModerationAction,
        row: EntityRecord
    ) {
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

        const nextActionKey = `${activeTab}:${action.key}:${userId}`;
        setActionKey(nextActionKey);
        try {
            if (action.key === 'kick') {
                await groupProfileRepository.kickGroupMember({
                    groupId: group.id,
                    userId,
                    endpoint
                });
            } else if (action.key === 'ban') {
                await groupProfileRepository.banGroupMember({
                    groupId: group.id,
                    userId,
                    endpoint
                });
            } else if (action.key === 'unban') {
                await groupProfileRepository.unbanGroupMember({
                    groupId: group.id,
                    userId,
                    endpoint
                });
            } else if (action.key === 'delete-invite') {
                await groupProfileRepository.deleteSentGroupInvite({
                    groupId: group.id,
                    userId,
                    endpoint
                });
            } else if (action.key === 'accept-request') {
                await groupProfileRepository.respondGroupJoinRequest({
                    groupId: group.id,
                    userId,
                    action: 'accept',
                    endpoint
                });
            } else if (action.key === 'reject-request') {
                await groupProfileRepository.respondGroupJoinRequest({
                    groupId: group.id,
                    userId,
                    action: 'reject',
                    endpoint
                });
            } else if (action.key === 'block-request') {
                await groupProfileRepository.respondGroupJoinRequest({
                    groupId: group.id,
                    userId,
                    action: 'reject',
                    block: true,
                    endpoint
                });
            } else if (action.key === 'delete-blocked') {
                await groupProfileRepository.deleteBlockedGroupRequest({
                    groupId: group.id,
                    userId,
                    endpoint
                });
            }
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

    return (
        <>
            <Dialog open={open} onOpenChange={onOpenChange}>
                <DialogContent className="sm:max-w-[min(92vw,64rem)]">
                    <DialogHeader>
                        <DialogTitle>
                            {t('dialog.group.actions.moderation_tools')}
                        </DialogTitle>
                        <DialogDescription>
                            {group.name || 'Group'}
                        </DialogDescription>
                    </DialogHeader>
                    <Tabs
                        value={activeTab}
                        onValueChange={setActiveTab}
                        className="min-h-0 gap-0"
                    >
                        <TabsList
                            variant="line"
                            className="h-auto w-full justify-start overflow-x-auto rounded-none border-b px-0 pb-1"
                        >
                            {moderationTabs.map((tab) => (
                                <TabsTrigger
                                    key={tab.value}
                                    value={tab.value}
                                    disabled={tab.disabled}
                                    className="flex-none rounded-none px-3"
                                >
                                    {tab.label}
                                </TabsTrigger>
                            ))}
                        </TabsList>
                        {bulkSelectable && selectedRows.length ? (
                            <GroupModerationBulkPanel
                                tabValue={activeTab as 'bans' | 'members'}
                                group={group}
                                selectedRows={selectedRows}
                                busy={bulkBusy}
                                progress={bulkProgress}
                                onClear={clearSelection}
                                onRemoveRow={(userId) =>
                                    toggleSelectedRow(userId, false)
                                }
                                onKick={runBulkKick}
                                onBan={runBulkBan}
                                onUnban={runBulkUnban}
                                onSaveNote={runBulkSaveNote}
                                onAddRoles={runBulkAddRoles}
                                onRemoveRoles={runBulkRemoveRoles}
                            />
                        ) : null}
                        {moderationTabs.map((tab) =>
                            tab.value === 'logs' ? (
                                <GroupModerationLogsPanel
                                    key={tab.value}
                                    active={activeTab === 'logs'}
                                    endpoint={endpoint}
                                    group={group}
                                    open={open}
                                />
                            ) : (
                                <GroupModerationTabPanel
                                    key={tab.value}
                                    actionKey={actionKey}
                                    activeTab={activeTab}
                                    error={error}
                                    group={group}
                                    loading={loading}
                                    onPageIndexChange={setPageIndex}
                                    onPageSizeChange={(
                                        nextPageSize: number
                                    ) => {
                                        setPageSize(nextPageSize);
                                        setPageIndex(0);
                                    }}
                                    onReload={() =>
                                        setReloadToken((value) => value + 1)
                                    }
                                    onRunAction={runModerationAction}
                                    onSearchChange={(nextSearch: string) => {
                                        setSearch(nextSearch);
                                        setPageIndex(0);
                                    }}
                                    onToggleAllVisible={toggleSelectedVisible}
                                    onToggleRow={toggleSelectedRow}
                                    pageIndex={pageIndex}
                                    pageSize={pageSize}
                                    rows={rows}
                                    search={search}
                                    selectable={BULK_SELECTABLE_TABS.has(
                                        tab.value
                                    )}
                                    selectedIds={selectedIds || undefined}
                                    tab={tab}
                                    toolbarExtra={
                                        tab.value === 'bans' &&
                                        hasGroupPermission(
                                            group,
                                            'group-bans-manage'
                                        ) ? (
                                            <Button
                                                type="button"
                                                size="sm"
                                                variant="outline"
                                                onClick={() =>
                                                    setBanImportOpen(true)
                                                }
                                            >
                                                <UploadIcon data-icon="inline-start" />
                                                {t(
                                                    'dialog.group_member_moderation.import_bans'
                                                )}
                                            </Button>
                                        ) : null
                                    }
                                />
                            )
                        )}
                    </Tabs>
                </DialogContent>
            </Dialog>
            <GroupModerationBanImportDialog
                open={banImportOpen}
                onOpenChange={setBanImportOpen}
                groupId={group.id}
                endpoint={endpoint}
                onImported={() => setReloadToken((value) => value + 1)}
            />
        </>
    );
}
