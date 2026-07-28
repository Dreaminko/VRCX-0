import { UsersIcon } from 'lucide-react';
import type { ReactElement } from 'react';

import { FadeInImage } from '@/components/media/FadeInImage';
import { cn } from '@/lib/utils';
import type { InventoryItemRecord } from '@/repositories/vrchatMediaRepository';
import { Button } from '@/ui/shadcn/button';

import { resolveProfileDecorationAssetUrls } from '../userDialogProfileAppearance';
import { UserDialogProfileDecorationImage } from './UserDialogProfileDecorationImage';

interface UserDialogHeaderMediaProps {
    bannerAlt: string;
    bannerColor: string;
    bannerUrl: string;
    iconFrame?: InventoryItemRecord;
    onBannerClick?: () => void;
    onOpenUserIcon: () => void;
    userIconLabel: string;
    userIconUrl: string;
}

export function UserDialogHeaderMedia({
    bannerAlt,
    bannerColor,
    bannerUrl,
    iconFrame,
    onBannerClick,
    onOpenUserIcon,
    userIconLabel,
    userIconUrl
}: UserDialogHeaderMediaProps): ReactElement {
    const { animatedUrl, staticUrl } =
        resolveProfileDecorationAssetUrls(iconFrame);
    const hasIconFrame = Boolean(animatedUrl || staticUrl);
    const bannerFallback = bannerColor ? (
        <span aria-hidden className="size-full" />
    ) : (
        <UsersIcon className="text-muted-foreground size-8" />
    );

    return (
        <div className="relative">
            <Button
                type="button"
                variant="ghost"
                disabled={!bannerUrl || !onBannerClick}
                onClick={onBannerClick}
                style={
                    bannerColor ? { backgroundColor: bannerColor } : undefined
                }
                className={cn(
                    'bg-muted aspect-[4/3] h-auto w-full overflow-hidden rounded-lg border p-0 disabled:pointer-events-none disabled:opacity-100',
                    bannerUrl ? 'cursor-pointer' : 'cursor-default'
                )}
            >
                {bannerUrl ? (
                    <FadeInImage
                        src={bannerUrl}
                        alt={bannerAlt}
                        className="size-full object-cover"
                        fallback={bannerFallback}
                    />
                ) : (
                    bannerFallback
                )}
            </Button>
            {userIconUrl ? (
                <div
                    className={cn(
                        'absolute z-30 size-16',
                        hasIconFrame ? 'right-4 bottom-4' : 'right-3 bottom-3'
                    )}
                >
                    <Button
                        type="button"
                        variant="ghost"
                        aria-label={userIconLabel}
                        title={userIconLabel}
                        className="bg-background/90 relative z-0 size-full overflow-hidden rounded-full border-2 border-white p-0 shadow-md"
                        onClick={onOpenUserIcon}
                    >
                        <FadeInImage
                            src={userIconUrl}
                            alt=""
                            className="size-full object-cover"
                        />
                    </Button>
                    {hasIconFrame ? (
                        <UserDialogProfileDecorationImage
                            item={iconFrame}
                            className="absolute -inset-4 z-10"
                            imageClassName="object-contain"
                        />
                    ) : null}
                </div>
            ) : null}
        </div>
    );
}
