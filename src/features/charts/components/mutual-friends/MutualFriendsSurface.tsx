import type { ReactNode } from 'react';

import { cn } from '@/lib/utils';

export function MutualFriendsSurface({
    className = '',
    children
}: {
    className?: string;
    children: ReactNode;
}) {
    return (
        <div
            className={cn(
                'bg-popover/70 ring-foreground/10 relative rounded-lg shadow-md ring-1',
                'before:pointer-events-none before:absolute before:inset-0 before:-z-1 before:rounded-[inherit] before:backdrop-blur-2xl before:backdrop-saturate-150',
                className
            )}
        >
            {children}
        </div>
    );
}
