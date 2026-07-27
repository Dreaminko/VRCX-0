import { SmileIcon, UserIcon } from 'lucide-react';
import type { ReactNode } from 'react';

import { getNotificationImageUrl } from '@/components/hosts/vrc-notification-center/notificationCenterUtils';
import { FadeInImage } from '@/components/media/FadeInImage';
import { cn } from '@/lib/utils';
import type { NotificationRow } from '@/repositories/notificationPersistenceRepository';
import { Avatar, AvatarFallback, AvatarImage } from '@/ui/shadcn/avatar';
import { Button } from '@/ui/shadcn/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/ui/shadcn/tooltip';

import { getDiscIcon } from '../notificationRowActions';
import type { NotificationViewModelEmoji } from '../notificationViewModel';

export function NotificationEmojiPreview({
    emoji,
    className = 'size-10',
    onClick
}: {
    className?: string;
    emoji: NotificationViewModelEmoji;
    onClick?: () => void;
}) {
    const fallback = (
        <span
            className={cn(
                'bg-muted text-muted-foreground flex shrink-0 items-center justify-center rounded-md',
                className
            )}
            title={emoji.name}
        >
            <SmileIcon className="size-4" />
        </span>
    );
    const preview = emoji.imageUrl ? (
        <FadeInImage
            src={emoji.imageUrl}
            alt={emoji.name}
            className={cn('shrink-0 rounded-md object-contain', className)}
            fallback={fallback}
        />
    ) : (
        fallback
    );
    if (!onClick) {
        return preview;
    }
    return (
        <button
            type="button"
            className="shrink-0 transition-transform ease-out active:scale-[0.97] motion-safe:duration-150"
            aria-label={emoji.name}
            onClick={onClick}
        >
            {preview}
        </button>
    );
}

export function NotificationPersonAvatar({
    notification,
    imageUrl,
    className = 'size-9'
}: {
    className?: string;
    imageUrl?: string;
    notification: NotificationRow;
}) {
    const resolvedImageUrl = imageUrl ?? getNotificationImageUrl(notification);
    return (
        <Avatar className={cn('shrink-0', className)}>
            {resolvedImageUrl ? (
                <AvatarImage src={resolvedImageUrl} alt="" />
            ) : null}
            <AvatarFallback>
                <UserIcon className="size-4" />
            </AvatarFallback>
        </Avatar>
    );
}

export function NotificationIconDisc({
    notification,
    imageUrl,
    className = 'size-9'
}: {
    className?: string;
    imageUrl?: string;
    notification: NotificationRow;
}) {
    const Icon = getDiscIcon(notification);
    const resolvedImageUrl = imageUrl ?? getNotificationImageUrl(notification);
    if (resolvedImageUrl) {
        return (
            <Avatar className={cn('shrink-0 rounded-md', className)}>
                <AvatarImage
                    src={resolvedImageUrl}
                    alt=""
                    className="rounded-md"
                />
                <AvatarFallback className="rounded-md">
                    <Icon className="size-4" />
                </AvatarFallback>
            </Avatar>
        );
    }
    return (
        <div
            className={cn(
                'bg-muted text-muted-foreground flex shrink-0 items-center justify-center rounded-md',
                className
            )}
        >
            <Icon className="size-4" />
        </div>
    );
}

export function NotificationActionButton({
    label,
    onClick,
    children
}: {
    children: ReactNode;
    label: string;
    onClick: () => void;
}) {
    return (
        <Tooltip>
            <TooltipTrigger
                render={
                    <Button
                        type="button"
                        variant="ghost"
                        size="icon-xs"
                        aria-label={label}
                        onClick={onClick}
                    >
                        {children}
                    </Button>
                }
            />
            <TooltipContent>{label}</TooltipContent>
        </Tooltip>
    );
}
