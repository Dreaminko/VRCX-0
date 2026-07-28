import { useState } from 'react';

import { FadeInImage } from '@/components/media/FadeInImage';
import { cn } from '@/lib/utils';
import type { InventoryItemRecord } from '@/repositories/vrchatMediaRepository';

import { resolveProfileDecorationAssetUrls } from '../userDialogProfileAppearance';

export function UserDialogProfileDecorationImage({
    item,
    className,
    imageClassName
}: {
    item: InventoryItemRecord | null | undefined;
    className?: string;
    imageClassName?: string;
}) {
    const { animatedUrl, staticUrl } = resolveProfileDecorationAssetUrls(item);
    const [failedAnimatedUrl, setFailedAnimatedUrl] = useState('');
    const animationFailed = failedAnimatedUrl === animatedUrl;
    if (!animatedUrl && !staticUrl) {
        return null;
    }

    return (
        <span
            aria-hidden="true"
            className={cn('pointer-events-none block', className)}
        >
            {animatedUrl && !animationFailed ? (
                <FadeInImage
                    src={animatedUrl}
                    alt=""
                    loading="lazy"
                    decoding="async"
                    fallback={null}
                    className={cn(
                        'size-full motion-reduce:hidden',
                        imageClassName
                    )}
                    onError={() => {
                        setFailedAnimatedUrl(animatedUrl);
                    }}
                />
            ) : null}
            {staticUrl ? (
                <FadeInImage
                    src={staticUrl}
                    alt=""
                    loading="lazy"
                    decoding="async"
                    className={cn(
                        'size-full',
                        animatedUrl &&
                            !animationFailed &&
                            'hidden motion-reduce:block',
                        imageClassName
                    )}
                />
            ) : null}
        </span>
    );
}
