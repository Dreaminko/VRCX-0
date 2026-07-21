import { useTranslation } from 'react-i18next';

import { BoopEmojiDialog } from '@/components/dialogs/BoopEmojiDialog';
import { NOTIFICATION_TYPES } from '@/repositories/notificationPersistenceRepository';
import { Button } from '@/ui/shadcn/button';
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuLabel,
    DropdownMenuTrigger
} from '@/ui/shadcn/dropdown-menu';

import type { NotificationRow } from '../notificationPageTypes';
import { sanitizeNotificationFilters } from '../notificationTableState';

const NOTIFICATION_TYPE_SECTIONS: {
    key: string;
    labelKey: string;
    types: string[];
}[] = [
    {
        key: 'social',
        labelKey: 'view.notification.feed.section.social',
        types: [
            'friendRequest',
            'ignoredFriendRequest',
            'invite',
            'requestInvite',
            'inviteResponse',
            'requestInviteResponse',
            'message',
            'boop'
        ]
    },
    {
        key: 'group',
        labelKey: 'view.notification.feed.section.group',
        types: [
            'groupChange',
            'group.announcement',
            'group.event.created',
            'group.informative',
            'group.invite',
            'group.joinRequest',
            'group.transfer',
            'group.queueReady',
            'event.announcement'
        ]
    },
    {
        key: 'moderation',
        labelKey: 'view.notification.feed.section.moderation',
        types: [
            'moderation.warning.group',
            'moderation.report.closed',
            'moderation.contentrestriction'
        ]
    },
    {
        key: 'other',
        labelKey: 'view.notification.feed.section.other',
        types: [
            'instance.closed',
            'economy.alert',
            'economy.received.gift',
            'badge.earned',
            'vrcplus.gift'
        ]
    }
];

function sectionedNotificationTypes() {
    const assigned = new Set(
        NOTIFICATION_TYPE_SECTIONS.flatMap((section) => section.types)
    );
    return NOTIFICATION_TYPE_SECTIONS.map((section) => ({
        ...section,
        types:
            section.key === 'other'
                ? [
                      ...section.types.filter((type) =>
                          NOTIFICATION_TYPES.includes(type)
                      ),
                      ...NOTIFICATION_TYPES.filter(
                          (type) => !assigned.has(type)
                      )
                  ]
                : section.types.filter((type) =>
                      NOTIFICATION_TYPES.includes(type)
                  )
    })).filter((section) => section.types.length > 0);
}

export function NotificationTypeFilterDropdown({
    value,
    onChange,
    getTypeLabel = (type: unknown) => String(type)
}: {
    getTypeLabel?: (type: string) => string;
    onChange: (value: string[]) => void;
    value: string[];
}) {
    const { t } = useTranslation();

    const activeTypes = value;
    const filterLabel = t('view.notification.filter_placeholder');
    const label = activeTypes.length
        ? `${filterLabel} (${activeTypes.length})`
        : filterLabel;
    const sections = sectionedNotificationTypes();

    function toggleType(type: string, checked: boolean) {
        const nextTypes = checked
            ? [...activeTypes, type]
            : activeTypes.filter((entry) => entry !== type);
        onChange(sanitizeNotificationFilters(nextTypes, NOTIFICATION_TYPES));
    }

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={
                    <Button
                        type="button"
                        variant="outline"
                        className="h-9 min-w-0 flex-1 basis-64 justify-start truncate"
                    >
                        {label}
                    </Button>
                }
            />
            <DropdownMenuContent
                align="start"
                className="max-h-96 w-80 overflow-y-auto"
            >
                {sections.map((section) => (
                    <DropdownMenuGroup key={section.key}>
                        <DropdownMenuLabel>
                            {t(section.labelKey)}
                        </DropdownMenuLabel>
                        {section.types.map((type) => (
                            <DropdownMenuCheckboxItem
                                key={type}
                                checked={activeTypes.includes(type)}
                                onCheckedChange={(checked) =>
                                    toggleType(type, checked)
                                }
                                onClick={(event) => event.preventDefault()}
                            >
                                {getTypeLabel(type)}
                            </DropdownMenuCheckboxItem>
                        ))}
                    </DropdownMenuGroup>
                ))}
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

export function BoopReplyDialog({
    request,
    isLocalUserVrcPlusSupporter,
    onOpenChange,
    onSend
}: {
    isLocalUserVrcPlusSupporter: boolean;
    onOpenChange: (open: boolean) => void;
    onSend: (
        notification: NotificationRow | null,
        emojiId: string
    ) => void | Promise<void>;
    request: NotificationRow | null;
}) {
    const open = Boolean(request);
    const notification = request || null;
    const displayName = notification?.senderUsername || 'this user';
    return (
        <BoopEmojiDialog
            open={open}
            isLocalUserVrcPlusSupporter={isLocalUserVrcPlusSupporter}
            targetLabel={displayName}
            sendDisabled={!notification?.senderUserId}
            onOpenChange={onOpenChange}
            onSend={(emojiId: string) => onSend(notification, emojiId)}
        />
    );
}
