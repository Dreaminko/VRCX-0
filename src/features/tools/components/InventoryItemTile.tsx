import { PackageIcon } from 'lucide-react';

import { MediaAssetTile } from './MediaAssetTile';
import type {
    MediaAssetAction,
    MediaAssetBadge,
    MediaPreviewOptions
} from './MediaAssetTile';

export function InventoryItemTile({
    title,
    description,
    timestamp,
    badges,
    imageUrl,
    alt,
    isCurrent,
    currentLabel,
    onPreview,
    primaryAction,
    menuActions,
    menuLabel
}: {
    title: string;
    description?: string;
    timestamp?: string;
    badges?: Array<MediaAssetBadge | null>;
    imageUrl?: string;
    alt?: string;
    isCurrent?: boolean;
    currentLabel?: string;
    onPreview?: (options?: MediaPreviewOptions) => void;
    primaryAction?: MediaAssetAction | null;
    menuActions?: Array<MediaAssetAction | null>;
    menuLabel?: string;
}) {
    const meta: Array<{ key: string; label: string; title: string }> = [];
    if (description) {
        meta.push({
            key: 'description',
            label: description,
            title: description
        });
    }
    if (timestamp) {
        meta.push({
            key: 'timestamp',
            label: timestamp,
            title: timestamp
        });
    }

    return (
        <MediaAssetTile
            title={title}
            meta={meta}
            badges={badges}
            imageUrl={imageUrl}
            alt={alt}
            isCurrent={isCurrent}
            currentLabel={currentLabel}
            imageFit="contain"
            placeholderIcon={PackageIcon}
            onPreview={onPreview}
            primaryAction={primaryAction}
            menuActions={menuActions}
            menuLabel={menuLabel}
            contentClassName="min-h-20"
        />
    );
}
