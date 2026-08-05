import {
    Globe2Icon,
    ImageOffIcon,
    PersonStandingIcon,
    Trash2Icon,
    UserRoundIcon,
    UsersRoundIcon
} from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { FadeInImage } from '@/components/media/FadeInImage';
import { formatRelativeTime } from '@/lib/dateTime';
import type { BrowseHistoryItemOutput } from '@/repositories/browseHistoryRepository';
import {
    openAvatarDialog,
    openGroupDialog,
    openUserDialog,
    openWorldDialog
} from '@/services/dialogService';
import { convertFileUrlToImageUrl } from '@/services/entityMediaService';
import { Button } from '@/ui/shadcn/button';

const iconByKind = {
    user: UserRoundIcon,
    world: Globe2Icon,
    avatar: PersonStandingIcon,
    group: UsersRoundIcon
};

function openHistoryItem(item: BrowseHistoryItemOutput) {
    const seedData = {
        id: item.entityId,
        name: item.title,
        displayName: item.title,
        authorName: item.subtitle,
        shortCode: item.subtitle,
        username: item.subtitle,
        thumbnailImageUrl: item.imageUrl,
        profilePicOverrideThumbnail: item.imageUrl,
        iconUrl: item.imageUrl
    };
    switch (item.entityKind) {
        case 'user':
            openUserDialog({
                userId: item.entityId,
                title: item.title,
                seedData
            });
            break;
        case 'world':
            openWorldDialog({
                worldId: item.entityId,
                title: item.title,
                seedData
            });
            break;
        case 'avatar':
            openAvatarDialog({
                avatarId: item.entityId,
                title: item.title,
                seedData
            });
            break;
        case 'group':
            openGroupDialog({
                groupId: item.entityId,
                title: item.title,
                seedData
            });
            break;
    }
}

export function BrowseHistoryCard({
    item,
    onRemove
}: {
    item: BrowseHistoryItemOutput;
    onRemove: (item: BrowseHistoryItemOutput) => void;
}) {
    const { t } = useTranslation();
    const Icon = iconByKind[item.entityKind];
    const imageUrl = convertFileUrlToImageUrl(item.imageUrl, 256);
    const title = item.title || item.entityId;
    const imageFallback = (
        <div className="bg-muted text-muted-foreground flex size-full items-center justify-center">
            {item.imageUrl ? (
                <ImageOffIcon className="size-5" />
            ) : (
                <Icon className="size-5" />
            )}
        </div>
    );

    return (
        <div
            className="border-border bg-card hover:bg-accent/40 group relative h-[104px] min-w-0 overflow-hidden rounded-xl border p-2"
        >
            <button
                type="button"
                className="focus-visible:ring-ring flex size-full min-w-0 cursor-pointer rounded-lg text-left outline-none focus-visible:ring-2"
                onClick={() => openHistoryItem(item)}
            >
                <div className="bg-muted size-[86px] shrink-0 overflow-hidden rounded-lg">
                    {imageUrl ? (
                        <FadeInImage
                            src={imageUrl}
                            alt=""
                            className="size-full object-cover"
                            fallback={imageFallback}
                        />
                    ) : (
                        imageFallback
                    )}
                </div>
                <div className="flex min-w-0 flex-1 flex-col justify-center px-3 pr-8">
                    <div className="flex items-center gap-1.5">
                        <Icon className="text-muted-foreground size-3.5 shrink-0" />
                        <span className="truncate text-sm font-medium">
                            {title}
                        </span>
                    </div>
                    <p className="text-muted-foreground mt-1 truncate text-xs">
                        {item.subtitle || item.entityId}
                    </p>
                    <p className="text-muted-foreground mt-2 truncate text-[11px] tabular-nums">
                        {formatRelativeTime(item.lastViewedAt)}
                        {item.viewCount > 1
                            ? ` · ${t('browse_history.view_count', { count: item.viewCount })}`
                            : ''}
                    </p>
                </div>
            </button>
            <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                className="absolute top-1.5 right-1.5 z-10 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100"
                aria-label={t('browse_history.actions.remove')}
                onClick={() => onRemove(item)}
            >
                <Trash2Icon />
            </Button>
        </div>
    );
}
