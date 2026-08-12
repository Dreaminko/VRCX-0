import type { ComponentProps, ReactNode } from 'react';

import { EmptyState, LoadingState } from '@/components/layout/PageScaffold';
import { cn } from '@/lib/utils';

function FavoritesEmptyState({
    title,
    description,
    className,
    ...props
}: ComponentProps<typeof EmptyState>) {
    return (
        <EmptyState
            {...props}
            variant="panel"
            title={title}
            description={description}
            className={cn('h-full min-h-60 border-0 p-6', className)}
        />
    );
}

function FavoritesLoadingState({ title }: { title?: ReactNode }) {
    return (
        <LoadingState
            variant="panel"
            label={title}
            className="h-full min-h-60 border-0 p-6"
        />
    );
}

export { FavoritesEmptyState, FavoritesLoadingState };
