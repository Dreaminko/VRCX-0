import { CheckIcon, MoreHorizontalIcon, Trash2Icon } from 'lucide-react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

import {
    openSender,
    shouldShowDeleteLog
} from '@/components/hosts/vrc-notification-center/notificationCenterUtils';
import { Location } from '@/components/Location';
import { FadeInImage } from '@/components/media/FadeInImage';
import { formatClock, formatDateFilter } from '@/lib/dateTime';
import { cn } from '@/lib/utils';
import type { NotificationRow as NotificationRecord } from '@/repositories/notificationPersistenceRepository';
import { convertFileUrlToImageUrl } from '@/services/entityMediaService';
import { Button } from '@/ui/shadcn/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger
} from '@/ui/shadcn/dropdown-menu';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/ui/shadcn/tooltip';

import {
    buildOrderedActions,
    canMarkNotificationSeen,
    getNotificationLinkIcon,
    PRIMARY_ACTION_KEYS,
    type NotificationRowActionHandlers
} from '../notificationRowActions';
import {
    NOTIFICATION_TYPE_LABEL_PREFIX,
    toNotificationViewModel
} from '../notificationViewModel';
import { useNotificationActorImage } from '../useNotificationActorImage';
import {
    NotificationActionButton,
    NotificationIconDisc,
    NotificationPersonAvatar
} from './NotificationRowParts';

export type NotificationFeedHandlers = NotificationRowActionHandlers & {
    onDeleteNotification(
        notification: NotificationRecord,
        options?: { skipConfirm?: boolean }
    ): void | Promise<void>;
    onMarkSeen(notification: NotificationRecord): void | Promise<void>;
    onOpenImagePreview(notification: NotificationRecord): void;
    onOpenLink(link: unknown): void;
};

export function NotificationRow({
    notification,
    currentUserId,
    canInviteFromCurrentLocation,
    handlers
}: {
    canInviteFromCurrentLocation: boolean;
    currentUserId?: string;
    handlers: NotificationFeedHandlers;
    notification: NotificationRecord;
}) {
    const { t } = useTranslation();
    const unknownLabel = t('view.notification.feed.unknown');
    const view = useMemo(
        () => toNotificationViewModel(notification, { unknownLabel }),
        [notification, unknownLabel]
    );
    const typeLabel = t(view.typeLabelKey, {
        defaultValue: view.typeLabelKey.slice(
            NOTIFICATION_TYPE_LABEL_PREFIX.length
        )
    });
    const actorName =
        view.actor.name || t('view.notification.feed.unknown_sender');
    const actorImageUrl = useNotificationActorImage(view.actor);
    const clockLabel = formatClock(view.createdAt);
    const absoluteLabel = formatDateFilter(view.createdAt, 'long');
    const LinkIcon = getNotificationLinkIcon(view.link?.href);

    const orderedActions = buildOrderedActions({
        notification,
        currentUserId,
        canInviteFromCurrentLocation,
        handlers,
        t
    });
    const inlineActions = orderedActions.slice(0, 2);
    const overflowActions = orderedActions.slice(2);
    const showMarkRead = view.unseen && canMarkNotificationSeen(notification);
    const showDelete = Boolean(shouldShowDeleteLog(notification));
    const hasMenu = showMarkRead || overflowActions.length > 0 || showDelete;

    const actorButton = (
        <button
            type="button"
            className="shrink-0 transition-transform ease-out active:scale-[0.97] motion-safe:duration-150"
            aria-label={actorName}
            onClick={() => openSender(notification, t)}
        >
            {view.actor.kind === 'user' ? (
                <NotificationPersonAvatar
                    notification={notification}
                    imageUrl={actorImageUrl}
                    className="size-9"
                />
            ) : (
                <NotificationIconDisc
                    notification={notification}
                    imageUrl={actorImageUrl}
                    className="size-9"
                />
            )}
        </button>
    );
    const locationLine = view.context ? (
        <span className="text-muted-foreground/80 min-w-0 truncate text-xs">
            <Location
                location={view.context.location}
                hint={view.context.worldName}
                grouphint={view.context.groupName}
                asButton={false}
            />
        </span>
    ) : null;
    const linkButton = view.link?.text ? (
        <Button
            type="button"
            variant="link"
            size="xs"
            className="h-auto max-w-56 justify-start p-0 text-xs font-normal no-underline transition-opacity duration-150 ease-out hover:no-underline hover:opacity-70"
            onClick={() => handlers.onOpenLink(view.link?.href)}
        >
            <LinkIcon data-icon="inline-start" />
            <span className="truncate">{view.link.text}</span>
        </Button>
    ) : null;
    const hasHeadline = Boolean(view.headline);

    return (
        <div
            className={cn(
                'group hover:bg-muted/40 flex items-start gap-3 rounded-lg py-2.5 pr-2 pl-1 transition-[background-color,opacity] duration-150 ease-out',
                view.expired && 'opacity-55 hover:opacity-100'
            )}
        >
            <span
                aria-hidden
                className={cn(
                    'mt-4 size-1.5 shrink-0 rounded-full',
                    view.unseen ? 'bg-primary' : 'bg-transparent'
                )}
            />
            {actorButton}
            <div className="flex min-w-0 flex-1 flex-col gap-1">
                <div className="flex min-w-0 items-center gap-2">
                    <button
                        type="button"
                        className="max-w-56 truncate text-left text-sm font-medium transition-opacity duration-150 ease-out hover:opacity-70"
                        onClick={() => openSender(notification, t)}
                    >
                        {actorName}
                    </button>
                    <span className="text-muted-foreground/70 shrink-0 truncate text-xs">
                        {typeLabel}
                    </span>
                    <div className="ml-auto flex shrink-0 items-center gap-2">
                        {view.expired ? (
                            <span className="border-border/60 text-muted-foreground/70 rounded-full border px-1.5 py-px text-[11px] leading-4">
                                {t('view.notification.feed.expired')}
                            </span>
                        ) : null}
                        <Tooltip>
                            <TooltipTrigger
                                render={
                                    <span className="text-muted-foreground/60 text-xs tabular-nums">
                                        {clockLabel}
                                    </span>
                                }
                            />
                            <TooltipContent>{absoluteLabel}</TooltipContent>
                        </Tooltip>
                    </div>
                </div>
                {hasHeadline ? (
                    <p className="text-foreground truncate text-sm font-medium">
                        {view.headline}
                    </p>
                ) : null}
                {view.body ? (
                    <p
                        className={cn(
                            'line-clamp-2 text-sm leading-snug break-words',
                            hasHeadline
                                ? 'text-muted-foreground'
                                : 'text-foreground/85'
                        )}
                    >
                        {view.body}
                    </p>
                ) : null}
                {locationLine || linkButton ? (
                    <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-0.5">
                        {locationLine}
                        {linkButton}
                    </div>
                ) : null}
            </div>
            <div className="flex shrink-0 items-center gap-1 pt-0.5">
                {view.media ? (
                    <button
                        type="button"
                        className="shrink-0 transition-transform ease-out active:scale-[0.97] motion-safe:duration-150"
                        aria-label={view.headline || typeLabel}
                        onClick={() =>
                            handlers.onOpenImagePreview(notification)
                        }
                    >
                        <FadeInImage
                            src={convertFileUrlToImageUrl(view.media, 64)}
                            alt=""
                            width={40}
                            height={40}
                            className="size-10 rounded-md object-cover"
                        />
                    </button>
                ) : null}
                {inlineActions.map((action) => (
                    <span
                        key={action.key}
                        className={cn(
                            'transition-opacity duration-150 ease-out',
                            !PRIMARY_ACTION_KEYS.has(action.key) &&
                                'opacity-0 group-hover:opacity-100 focus-within:opacity-100'
                        )}
                    >
                        <NotificationActionButton
                            label={action.label}
                            onClick={action.onClick}
                        >
                            <action.Icon data-icon="icon" />
                        </NotificationActionButton>
                    </span>
                ))}
                {hasMenu ? (
                    <DropdownMenu>
                        <DropdownMenuTrigger
                            render={
                                <Button
                                    type="button"
                                    variant="ghost"
                                    size="icon-xs"
                                    aria-label={t(
                                        'side_panel.notification_center.more_actions'
                                    )}
                                >
                                    <MoreHorizontalIcon data-icon="icon" />
                                </Button>
                            }
                        />
                        <DropdownMenuContent align="end">
                            <DropdownMenuGroup>
                                {showMarkRead ? (
                                    <DropdownMenuItem
                                        onClick={() =>
                                            handlers.onMarkSeen(notification)
                                        }
                                    >
                                        <CheckIcon data-icon="inline-start" />
                                        {t(
                                            'view.notification.action.mark_seen'
                                        )}
                                    </DropdownMenuItem>
                                ) : null}
                                {overflowActions.map((action) => (
                                    <DropdownMenuItem
                                        key={action.key}
                                        onClick={action.onClick}
                                    >
                                        <action.Icon data-icon="inline-start" />
                                        {action.label}
                                    </DropdownMenuItem>
                                ))}
                            </DropdownMenuGroup>
                            {showDelete ? (
                                <>
                                    {showMarkRead ||
                                    overflowActions.length > 0 ? (
                                        <DropdownMenuSeparator />
                                    ) : null}
                                    <DropdownMenuGroup>
                                        <DropdownMenuItem
                                            variant="destructive"
                                            onClick={(event) =>
                                                handlers.onDeleteNotification(
                                                    notification,
                                                    {
                                                        skipConfirm:
                                                            event.shiftKey
                                                    }
                                                )
                                            }
                                        >
                                            <Trash2Icon data-icon="inline-start" />
                                            {t(
                                                'view.notification.actions.delete_log'
                                            )}
                                        </DropdownMenuItem>
                                    </DropdownMenuGroup>
                                </>
                            ) : null}
                        </DropdownMenuContent>
                    </DropdownMenu>
                ) : null}
            </div>
        </div>
    );
}
