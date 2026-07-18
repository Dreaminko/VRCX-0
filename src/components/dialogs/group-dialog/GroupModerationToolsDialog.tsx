import { useTranslation } from 'react-i18next';

import type { GroupProfileRecord } from '@/domain/entities/profileEntities';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle
} from '@/ui/shadcn/dialog';

import { GroupModerationWorkspace } from './GroupModerationWorkspace';

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

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-[min(94vw,76rem)]">
                <DialogHeader>
                    <DialogTitle>
                        {t('dialog.group.actions.moderation_tools')}
                    </DialogTitle>
                    <DialogDescription>
                        {group.name || 'Group'}
                    </DialogDescription>
                </DialogHeader>
                {open ? (
                    <GroupModerationWorkspace
                        group={group}
                        endpoint={endpoint}
                    />
                ) : null}
            </DialogContent>
        </Dialog>
    );
}
