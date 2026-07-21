import { UserIcon } from 'lucide-react';
import type { ReactNode } from 'react';

import { getNotificationImageUrl } from '@/components/hosts/vrc-notification-center/notificationCenterUtils';
import { cn } from '@/lib/utils';
import type { NotificationRow } from '@/repositories/notificationPersistenceRepository';
import { Avatar, AvatarFallback, AvatarImage } from '@/ui/shadcn/avatar';
import { Button } from '@/ui/shadcn/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/ui/shadcn/tooltip';

import { getDiscIcon } from '../notificationRowActions';

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
