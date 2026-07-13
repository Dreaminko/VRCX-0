import { cn } from '@/lib/utils';

import { resolveFeedTypeMeta } from '../feedRows';

function FeedTypeIndicator({ label, type }: { label: string; type: unknown }) {
    const meta = resolveFeedTypeMeta(type);
    return (
        <span className="inline-flex min-w-0 items-center gap-1.5">
            {meta.className ? (
                <span
                    aria-hidden="true"
                    className={cn(
                        'size-2 shrink-0 rounded-full',
                        meta.className
                    )}
                />
            ) : null}
            <span className="truncate text-sm">{label}</span>
        </span>
    );
}

export { FeedTypeIndicator };
