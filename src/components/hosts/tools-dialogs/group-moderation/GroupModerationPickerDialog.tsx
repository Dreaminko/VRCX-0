import { LoaderCircleIcon, ShieldUserIcon } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { hasAnyGroupModerationPermission } from '@/components/dialogs/group-dialog/groupDialogUtils';
import { GroupModerationToolsDialog } from '@/components/dialogs/group-dialog/GroupModerationToolsDialog';
import { FadeInImage } from '@/components/media/FadeInImage';
import type { GroupProfileRecord } from '@/domain/entities/profileEntities';
import { userFacingErrorMessage } from '@/lib/errorDisplay';
import type { UserGroupsOverviewGroup } from '@/platform/tauri/bindings';
import groupProfileRepository from '@/repositories/groupProfileRepository';
import { convertFileUrlToImageUrl } from '@/services/entityMediaService';
import { Button } from '@/ui/shadcn/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle
} from '@/ui/shadcn/dialog';
import {
    Empty,
    EmptyDescription,
    EmptyHeader,
    EmptyTitle
} from '@/ui/shadcn/empty';
import { ScrollArea } from '@/ui/shadcn/scroll-area';

type LoadStatus = 'idle' | 'loading' | 'ready' | 'error';

type GroupModerationPickerDialogProps = {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    currentUserId: string;
    endpoint: string;
};

function GroupIcon({ group }: { group: UserGroupsOverviewGroup }) {
    const iconUrl = group.iconUrl
        ? convertFileUrlToImageUrl(group.iconUrl, 128)
        : '';
    return (
        <span className="bg-muted flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-md border">
            {iconUrl ? (
                <FadeInImage
                    src={iconUrl}
                    alt=""
                    className="size-full object-cover"
                    fallback={
                        <ShieldUserIcon className="text-muted-foreground size-4" />
                    }
                />
            ) : (
                <ShieldUserIcon className="text-muted-foreground size-4" />
            )}
        </span>
    );
}

export function GroupModerationPickerDialog({
    open,
    onOpenChange,
    currentUserId,
    endpoint
}: GroupModerationPickerDialogProps) {
    const { t } = useTranslation();
    const [status, setStatus] = useState<LoadStatus>('idle');
    const [error, setError] = useState('');
    const [groups, setGroups] = useState<UserGroupsOverviewGroup[]>([]);
    const [permissionsDegraded, setPermissionsDegraded] = useState(false);
    const [reloadToken, setReloadToken] = useState(0);
    const [selectedGroup, setSelectedGroup] =
        useState<GroupProfileRecord | null>(null);
    const [selectLoadingGroupId, setSelectLoadingGroupId] = useState('');

    useEffect(() => {
        if (!open) {
            setSelectedGroup(null);
            setSelectLoadingGroupId('');
            return;
        }

        if (!currentUserId) {
            setStatus('ready');
            setGroups([]);
            setPermissionsDegraded(false);
            return;
        }

        let active = true;
        setStatus('loading');
        setError('');
        groupProfileRepository
            .getUserGroupsOverview({ userId: currentUserId, endpoint })
            .then((output) => {
                if (!active) {
                    return;
                }
                setGroups(
                    output.groups.filter((group) =>
                        hasAnyGroupModerationPermission(group.permissions)
                    )
                );
                setPermissionsDegraded(output.permissionsDegraded);
                setStatus('ready');
            })
            .catch((requestError: unknown) => {
                if (!active) {
                    return;
                }
                setStatus('error');
                setError(
                    userFacingErrorMessage(
                        requestError,
                        t(
                            'host.tools_dialogs.toast.failed_to_load_moderatable_groups'
                        )
                    )
                );
            });

        return () => {
            active = false;
        };
    }, [open, currentUserId, endpoint, reloadToken, t]);

    async function selectGroup(group: UserGroupsOverviewGroup) {
        if (selectLoadingGroupId) {
            return;
        }
        setSelectLoadingGroupId(group.groupId);
        try {
            const profile = await groupProfileRepository.getGroupProfile({
                groupId: group.groupId,
                endpoint
            });
            setSelectedGroup(profile);
        } catch (requestError) {
            toast.error(
                userFacingErrorMessage(
                    requestError,
                    t(
                        'host.tools_dialogs.toast.failed_to_open_group_moderation'
                    )
                )
            );
        } finally {
            setSelectLoadingGroupId('');
        }
    }

    return (
        <>
            <Dialog
                open={open && !selectedGroup}
                onOpenChange={(nextOpen) => {
                    if (!nextOpen) {
                        onOpenChange(false);
                    }
                }}
            >
                <DialogContent className="sm:max-w-[min(92vw,32rem)]">
                    <DialogHeader>
                        <DialogTitle>
                            {t('view.tools.group.moderation')}
                        </DialogTitle>
                        <DialogDescription>
                            {t(
                                'host.tools_dialogs.group_moderation.picker_description'
                            )}
                        </DialogDescription>
                    </DialogHeader>
                    {permissionsDegraded ? (
                        <p className="text-muted-foreground text-xs">
                            {t(
                                'host.tools_dialogs.group_moderation.permissions_degraded'
                            )}
                        </p>
                    ) : null}
                    {status === 'loading' ? (
                        <div className="text-muted-foreground flex min-h-[160px] items-center justify-center gap-2 text-sm">
                            <LoaderCircleIcon className="size-4 animate-spin" />
                            <span>
                                {t(
                                    'host.tools_dialogs.group_moderation.loading'
                                )}
                            </span>
                        </div>
                    ) : status === 'error' ? (
                        <div className="text-destructive flex min-h-[160px] flex-col items-center justify-center gap-2 text-center text-sm">
                            <span>{error}</span>
                            <Button
                                type="button"
                                size="sm"
                                variant="outline"
                                onClick={() =>
                                    setReloadToken((token) => token + 1)
                                }
                            >
                                {t('common.action.retry')}
                            </Button>
                        </div>
                    ) : groups.length ? (
                        <ScrollArea className="max-h-[50vh]">
                            <div className="flex flex-col gap-1 p-1">
                                {groups.map((group) => (
                                    <Button
                                        key={group.groupId}
                                        type="button"
                                        variant="ghost"
                                        disabled={Boolean(selectLoadingGroupId)}
                                        className="h-auto justify-start gap-3 px-3 py-2"
                                        onClick={() => selectGroup(group)}
                                    >
                                        <GroupIcon group={group} />
                                        <span className="min-w-0 flex-1 text-left">
                                            <span className="block truncate font-semibold">
                                                {group.name || group.groupId}
                                            </span>
                                            {typeof group.memberCount ===
                                            'number' ? (
                                                <span className="text-muted-foreground block truncate text-xs">
                                                    {t(
                                                        'host.tools_dialogs.group_moderation.member_count',
                                                        {
                                                            count: group.memberCount
                                                        }
                                                    )}
                                                </span>
                                            ) : null}
                                        </span>
                                        {selectLoadingGroupId ===
                                        group.groupId ? (
                                            <LoaderCircleIcon className="size-4 shrink-0 animate-spin" />
                                        ) : null}
                                    </Button>
                                ))}
                            </div>
                        </ScrollArea>
                    ) : (
                        <Empty className="min-h-[160px] border-0">
                            <EmptyHeader>
                                <EmptyTitle>
                                    {t(
                                        'host.tools_dialogs.group_moderation.empty_title'
                                    )}
                                </EmptyTitle>
                                <EmptyDescription>
                                    {t(
                                        'host.tools_dialogs.group_moderation.empty_description'
                                    )}
                                </EmptyDescription>
                            </EmptyHeader>
                        </Empty>
                    )}
                </DialogContent>
            </Dialog>
            {selectedGroup ? (
                <GroupModerationToolsDialog
                    open={Boolean(selectedGroup)}
                    onOpenChange={(nextOpen) => {
                        if (!nextOpen) {
                            setSelectedGroup(null);
                        }
                    }}
                    group={selectedGroup}
                    endpoint={endpoint}
                />
            ) : null}
        </>
    );
}
