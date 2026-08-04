import {
    MoreHorizontalIcon,
    ImageIcon,
    type LucideIcon
} from 'lucide-react';
import type { ComponentProps, ReactNode } from 'react';

import { FadeInImage } from '@/components/media/FadeInImage';
import { TILE_MOTION, TILE_SELECTED } from '@/lib/selectableTile';
import { cn } from '@/lib/utils';
import { Badge } from '@/ui/shadcn/badge';
import { Button } from '@/ui/shadcn/button';
import { Card, CardContent } from '@/ui/shadcn/card';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuTrigger
} from '@/ui/shadcn/dropdown-menu';

export type MediaAssetBadge = {
    key?: string;
    label: string;
    variant?: ComponentProps<typeof Badge>['variant'];
};

export type MediaAssetAction = {
    key?: string;
    label: string;
    icon?: LucideIcon;
    destructive?: boolean;
    disabled?: boolean;
    variant?: ComponentProps<typeof Button>['variant'];
    onClick?: () => void;
    onSelect?: () => void;
};

export type MediaAssetMeta = {
    key?: string;
    label: string;
    title?: string;
};

export type MediaPreviewOptions = {
    id?: string;
    url: string;
    title: string;
};

export function shortAssetId(value: unknown) {
    const text = String(value || '').trim();
    if (!text) {
        return '';
    }
    if (text.length <= 18) {
        return text;
    }
    return `${text.slice(0, 10)}...${text.slice(-6)}`;
}

function renderIcon(Icon?: LucideIcon) {
    return Icon ? <Icon data-icon="inline-start" /> : null;
}

function TileActionsMenu({
    actions = [],
    label
}: {
    actions?: Array<MediaAssetAction | null>;
    label?: string;
}) {
    const visibleActions = actions.filter(
        (action): action is MediaAssetAction => Boolean(action)
    );
    if (!visibleActions.length) {
        return null;
    }

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={
                    <Button
                        type="button"
                        variant="secondary"
                        size="icon-sm"
                        className="bg-background/70 backdrop-blur"
                        aria-label={label}
                    >
                        <MoreHorizontalIcon data-icon="inline-start" />
                    </Button>
                }
            />
            <DropdownMenuContent align="end" className="w-44">
                <DropdownMenuGroup>
                    {visibleActions.map((action) => (
                        <DropdownMenuItem
                            key={action.key || action.label}
                            variant={
                                action.destructive ? 'destructive' : 'default'
                            }
                            disabled={action.disabled}
                            closeOnClick={false}
                            onClick={(event) => {
                                event.preventDefault();
                                action.onSelect?.();
                            }}
                        >
                            {renderIcon(action.icon)}
                            {action.label}
                        </DropdownMenuItem>
                    ))}
                </DropdownMenuGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

export function MediaAssetTile({
    title,
    subtitle,
    meta,
    badges,
    imageUrl,
    alt,
    aspectClass = 'aspect-square',
    imageFit = 'cover',
    imagePosition = 'center',
    isCurrent = false,
    currentLabel,
    menuLabel,
    placeholderIcon: PlaceholderIcon = ImageIcon,
    renderMedia,
    onPreview,
    onMediaClick,
    mediaHoverLabel,
    primaryAction,
    menuActions,
    className,
    contentClassName,
    hideContent = false
}: {
    title?: unknown;
    subtitle?: unknown;
    meta?: MediaAssetMeta | MediaAssetMeta[];
    badges?: Array<MediaAssetBadge | null>;
    imageUrl?: string;
    alt?: string;
    aspectClass?: string;
    imageFit?: 'cover' | 'contain';
    imagePosition?: 'center' | 'top';
    isCurrent?: boolean;
    currentLabel?: string;
    menuLabel?: string;
    placeholderIcon?: LucideIcon;
    renderMedia?: ((options: { className: string }) => ReactNode) | null;
    onPreview?: () => void;
    onMediaClick?: () => void;
    mediaHoverLabel?: string;
    primaryAction?: MediaAssetAction | null;
    menuActions?: Array<MediaAssetAction | null>;
    className?: string;
    contentClassName?: string;
    hideContent?: boolean;
}) {
    const safeTitle = String(title || '').trim();
    const safeSubtitle = String(subtitle || '').trim();
    const safeMeta = (Array.isArray(meta) ? meta : meta ? [meta] : []);
    const safeBadges = (badges || []).filter(
        (badge): badge is MediaAssetBadge => Boolean(badge)
    );
    const resolvedPrimaryAction = primaryAction?.label
        ? primaryAction
        : null;
    const handleMediaClick = onMediaClick || onPreview;
    const imageClassName = cn(
        'size-full',
        imageFit === 'contain' ? 'object-contain' : 'object-cover',
        imagePosition === 'top' && 'object-top'
    );

    return (
        <Card
            size="sm"
            className={cn(
                'group/tile gap-0 overflow-hidden rounded-lg py-0 transition-colors data-[size=sm]:gap-0 data-[size=sm]:py-0',
                isCurrent && TILE_SELECTED,
                className
            )}
        >
            <div className="relative">
                <Button
                    type="button"
                    variant="ghost"
                    className={cn(
                        'block h-auto w-full rounded-none p-0',
                        TILE_MOTION
                    )}
                    onClick={
                        imageUrl || renderMedia ? handleMediaClick : undefined
                    }
                >
                    <div
                        className={cn(
                            'bg-muted/30 text-muted-foreground flex w-full items-center justify-center overflow-hidden',
                            aspectClass
                        )}
                    >
                        {renderMedia ? (
                            renderMedia({
                                className: imageClassName
                            })
                        ) : imageUrl ? (
                            <FadeInImage
                                src={imageUrl}
                                alt={alt || safeTitle}
                                loading="lazy"
                                className={imageClassName}
                                fallback={
                                    <PlaceholderIcon className="size-8" />
                                }
                            />
                        ) : (
                            <PlaceholderIcon className="size-8" />
                        )}
                    </div>
                </Button>
                <div className="pointer-events-none absolute top-2 left-2 flex flex-wrap gap-1">
                    {isCurrent && currentLabel ? (
                        <Badge variant="secondary" className="bg-background/80">
                            {currentLabel}
                        </Badge>
                    ) : null}
                    {safeBadges.map((badge) => (
                        <Badge
                            key={badge.key || badge.label}
                            variant={badge.variant || 'outline'}
                            className="bg-background/80"
                        >
                            {badge.label}
                        </Badge>
                    ))}
                </div>
                {mediaHoverLabel ? (
                    <div className="bg-background/85 text-foreground pointer-events-none absolute top-2 left-2 max-w-[calc(100%-3rem)] -translate-y-1 rounded-sm px-1.5 py-0.5 text-xs font-medium opacity-0 backdrop-blur-[1px] transition-[opacity,transform] ease-out group-focus-within/tile:translate-y-0 group-focus-within/tile:opacity-100 group-hover/tile:translate-y-0 group-hover/tile:opacity-100 motion-reduce:translate-y-0 motion-reduce:transition-opacity">
                        {mediaHoverLabel}
                    </div>
                ) : null}
                <div className="absolute top-2 right-2">
                    <TileActionsMenu actions={menuActions} label={menuLabel} />
                </div>
            </div>
            {hideContent ? null : (
                <CardContent
                    className={cn(
                        'flex min-h-20 items-start gap-2 p-2.5',
                        contentClassName
                    )}
                >
                    <div className="min-w-0 flex-1">
                        {safeTitle ? (
                            <div
                                className="truncate text-sm font-medium"
                                title={safeTitle}
                            >
                                {safeTitle}
                            </div>
                        ) : null}
                        {safeSubtitle ? (
                            <div
                                className="text-muted-foreground truncate font-mono text-xs"
                                title={safeSubtitle}
                            >
                                {safeSubtitle}
                            </div>
                        ) : null}
                        {safeMeta.map((item) => (
                            <div
                                key={item.key || item.label}
                                className="text-muted-foreground truncate text-xs"
                                title={item.title || item.label}
                            >
                                {item.label}
                            </div>
                        ))}
                    </div>
                    {resolvedPrimaryAction ? (
                        <Button
                            type="button"
                            variant={resolvedPrimaryAction.variant || 'outline'}
                            size="sm"
                            className="shrink-0"
                            disabled={resolvedPrimaryAction.disabled}
                            onClick={resolvedPrimaryAction.onClick}
                        >
                            {renderIcon(resolvedPrimaryAction.icon)}
                            {resolvedPrimaryAction.label}
                        </Button>
                    ) : null}
                </CardContent>
            )}
        </Card>
    );
}
