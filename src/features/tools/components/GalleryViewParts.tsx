import {
    EmptyState as AppEmptyState,
    LoadingState as AppLoadingState
} from '@/components/layout/PageScaffold';
import { cn } from '@/lib/utils';

export function EmptyState({
    title,
    description,
    className,
    children,
    ...props
}: ComponentProps<typeof AppEmptyState>) {
    return (
        <AppEmptyState
            {...props}
            className={cn('min-h-72', className)}
            title={title}
            description={description}
        >
            {children}
        </AppEmptyState>
    );
}

export function LoadingState() {
    return <AppLoadingState className="min-h-72" />;
}
import type { ComponentProps } from 'react';
