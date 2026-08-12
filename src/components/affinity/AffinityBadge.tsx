import { HeartIcon, StarIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/utils';

type AffinityBadgeProps = {
    isFriend?: boolean;
    isFavorite?: boolean;
    iconOnly?: boolean;
    className?: string;
};

export function AffinityBadge({
    isFriend,
    isFavorite,
    iconOnly = false,
    className
}: AffinityBadgeProps) {
    const { t } = useTranslation();

    if (!isFriend) {
        return null;
    }

    const favorite = Boolean(isFavorite);
    const Icon = favorite ? StarIcon : HeartIcon;
    const label = t(
        favorite ? 'common.affinity.favorite' : 'common.affinity.friend'
    );

    return (
        <span
            aria-label={iconOnly ? label : undefined}
            role={iconOnly ? 'img' : undefined}
            className={cn(
                'inline-flex h-[18px] shrink-0 items-center gap-1 rounded-md px-1.5 text-[0.7rem] font-medium',
                iconOnly &&
                    'size-4 justify-center rounded-none bg-transparent p-0',
                iconOnly && favorite && 'text-amber-400/70',
                iconOnly && !favorite && 'text-rose-300/70',
                !iconOnly && favorite &&
                    'bg-amber-500/10 text-amber-300',
                !iconOnly && !favorite && 'bg-rose-500/10 text-rose-300',
                className
            )}
        >
            <Icon
                className={cn(
                    'size-3 shrink-0',
                    !iconOnly && 'fill-current'
                )}
            />
            {!iconOnly ? label : null}
        </span>
    );
}
