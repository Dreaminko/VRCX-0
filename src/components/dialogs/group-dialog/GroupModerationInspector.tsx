import { CopyIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import type {
    EntityRecord,
    GroupProfileRecord
} from '@/domain/entities/profileEntities';
import { formatDateFilter } from '@/lib/dateTime';
import { copyTextToClipboard } from '@/services/clipboardService';
import { openUserDialog } from '@/services/dialogService';
import { userImage } from '@/services/entityMediaService';
import { Avatar, AvatarFallback, AvatarImage } from '@/ui/shadcn/avatar';
import { Badge } from '@/ui/shadcn/badge';
import { Button } from '@/ui/shadcn/button';

import {
    getGroupModerationActions,
    moderationRowDate,
    moderationRowLabel,
    moderationRowNote,
    moderationRowRoleIds,
    moderationRowStatus,
    moderationRowUserId,
    type GroupModerationAction
} from './groupModerationRows';
import {
    getGroupRoleNameMap,
    type GroupModerationTabValue
} from './groupDialogUtils';

function isRecord(value: unknown): value is EntityRecord {
    return Boolean(value && typeof value === 'object');
}

function record(value: unknown): EntityRecord | null {
    return isRecord(value) ? value : null;
}

export function GroupModerationInspector({
    row,
    group,
    tabValue,
    actionKey,
    onRunAction
}: {
    row: EntityRecord | null;
    group: GroupProfileRecord;
    tabValue: GroupModerationTabValue;
    actionKey: string;
    onRunAction: (action: GroupModerationAction, row: EntityRecord) => void;
}) {
    const { t } = useTranslation();

    if (!row) {
        return (
            <div className="w-80 shrink-0 border-l pl-4">
                <div className="text-muted-foreground flex h-40 items-center justify-center text-center text-sm">
                    {t('dialog.group_member_moderation.inspector_empty')}
                </div>
            </div>
        );
    }

    const user = record(row.user);
    const userId = moderationRowUserId(row);
    const label = moderationRowLabel(row);
    const date = moderationRowDate(row);
    const status = moderationRowStatus(row);
    const roleNames = getGroupRoleNameMap(group);
    const roleIds = moderationRowRoleIds(row);
    const roleLabels = roleIds.map((roleId) => roleNames.get(roleId) || 'Role');
    const note = moderationRowNote(row);
    const actions = getGroupModerationActions(tabValue, row, t);
    const imageUrl = userImage(user ?? row, true, '128');

    function handleCopyUserId() {
        void copyTextToClipboard(userId, {
            successMessage: t('dialog.group.dynamic.value_copied', {
                value: 'User ID'
            })
        });
    }

    function handleViewFullProfile() {
        openUserDialog({ userId, title: label, seedData: user });
    }

    return (
        <div className="w-80 shrink-0 border-l pl-4">
            <div className="flex items-center gap-3">
                <Avatar size="lg">
                    {imageUrl ? (
                        <AvatarImage src={imageUrl} alt={label} />
                    ) : null}
                    <AvatarFallback>{label.slice(0, 1)}</AvatarFallback>
                </Avatar>
                <div className="min-w-0">
                    <div className="truncate font-medium">{label}</div>
                    {userId ? (
                        <span className="flex min-w-0 items-center gap-1">
                            <span className="text-muted-foreground truncate font-mono text-xs">
                                {userId}
                            </span>
                            <Button
                                type="button"
                                aria-label={t('dialog.user.info.copy_id')}
                                title={t('dialog.user.info.copy_id')}
                                size="icon-xs"
                                variant="ghost"
                                onClick={handleCopyUserId}
                            >
                                <CopyIcon data-icon="inline-start" />
                            </Button>
                        </span>
                    ) : null}
                </div>
            </div>

            <div className="mt-4 space-y-3 text-sm">
                <div>
                    <div className="text-muted-foreground text-xs">
                        {t('dialog.group_member_moderation.roles')}
                    </div>
                    <div className="mt-1 flex flex-wrap gap-1">
                        {roleLabels.length ? (
                            roleLabels.map((roleLabel, index) => (
                                <Badge
                                    key={`${roleLabel}:${index}`}
                                    variant="outline"
                                >
                                    {roleLabel}
                                </Badge>
                            ))
                        ) : (
                            <span className="text-muted-foreground">—</span>
                        )}
                    </div>
                </div>
                <div>
                    <div className="text-muted-foreground text-xs">
                        {t('dialog.group.label.status')}
                    </div>
                    <div>{status}</div>
                </div>
                <div>
                    <div className="text-muted-foreground text-xs">
                        {t('dialog.group.label.date')}
                    </div>
                    <div>{date ? formatDateFilter(date, 'long') : '—'}</div>
                </div>
                {note ? (
                    <div>
                        <div className="text-muted-foreground text-xs">
                            {t('dialog.group_member_moderation.description')}
                        </div>
                        <div className="text-muted-foreground text-xs whitespace-normal">
                            {note}
                        </div>
                    </div>
                ) : null}
            </div>

            {actions.length ? (
                <div className="mt-4 flex flex-wrap gap-2">
                    {actions.map((action) => {
                        const nextActionKey = `${tabValue}:${action.key}:${userId}`;
                        return (
                            <Button
                                key={action.key}
                                type="button"
                                size="sm"
                                variant={
                                    action.destructive
                                        ? 'outline'
                                        : 'secondary'
                                }
                                disabled={Boolean(actionKey)}
                                onClick={() => onRunAction(action, row)}
                            >
                                {actionKey === nextActionKey
                                    ? '...'
                                    : action.label}
                            </Button>
                        );
                    })}
                </div>
            ) : null}

            <Button
                type="button"
                size="sm"
                variant="ghost"
                className="mt-4 w-full"
                disabled={!userId}
                onClick={handleViewFullProfile}
            >
                {t('dialog.group_member_moderation.view_full_profile')}
            </Button>
        </div>
    );
}
