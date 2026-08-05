import {
    Globe2Icon,
    ImageOffIcon,
    PersonStandingIcon,
    Trash2Icon,
    UserRoundIcon,
    UsersRoundIcon
} from 'lucide-react';
import { memo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { FadeInImage } from '@/components/media/FadeInImage';
import { formatClock } from '@/lib/dateTime';
import { cn } from '@/lib/utils';
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

export const BrowseHistoryCard = memo(function BrowseHistoryCard({
    item,
    onRemove
}: {
    item: BrowseHistoryItemOutput;
    onRemove: (item: BrowseHistoryItemOutput) => Promise<boolean>;
}) {
    const { t } = useTranslation();
    const [removing, setRemoving] = useState(false);
    const Icon = iconByKind[item.entityKind];
    const imageUrl = convertFileUrlToImageUrl(item.imageUrl, 128);
    const title = item.title || t(`browse_history.unknown.${item.entityKind}`);
    const imageFallback = (
        <div className="bg-muted text-muted-foreground flex size-full items-center justify-center">
            {item.imageUrl ? (
                <ImageOffIcon className="size-4" />
            ) : (
                <Icon className="size-4" />
            )}
        </div>
    );

    return (
        <div
            className={cn(
                'border-border bg-card group hover:bg-accent/40 relative h-16 min-w-0 overflow-hidden rounded-lg border p-2 transition-[background-color,opacity] duration-150 ease-out',
                removing && 'pointer-events-none opacity-0'
            )}
        >
            <button
                type="button"
                className="focus-visible:ring-ring flex size-full min-w-0 cursor-pointer items-center gap-2.5 rounded-md text-left transition-transform duration-100 ease-out outline-none focus-visible:ring-2 active:scale-[0.99]"
                onClick={() => openHistoryItem(item)}
            >
                <div className="bg-muted size-12 shrink-0 overflow-hidden rounded-md">
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
                <div className="flex min-w-0 flex-1 flex-col gap-0.5 pr-6">
                    <div className="flex items-center gap-1.5">
                        <Icon className="text-muted-foreground size-3 shrink-0" />
                        <span className="truncate text-[13px] leading-tight font-medium">
                            {title}
                        </span>
                    </div>
                    <p className="text-muted-foreground truncate text-[11px] leading-tight tabular-nums">
                        {formatClock(item.lastViewedAt)}
                        {item.viewCount > 1 ? ` · ×${item.viewCount}` : ''}
                    </p>
                </div>
            </button>
            <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                className="absolute top-1 right-1 z-10 opacity-0 transition-opacity duration-150 ease-out group-focus-within:opacity-100 group-hover:opacity-100"
                aria-label={t('browse_history.actions.remove')}
                onClick={() => {
                    setRemoving(true);
                    void onRemove(item).then((removed) => {
                        if (!removed) {
                            setRemoving(false);
                        }
                    });
                }}
            >
                <Trash2Icon />
            </Button>
        </div>
    );
});
