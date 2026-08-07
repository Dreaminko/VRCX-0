import type * as React from 'react';

import { cn } from '@/lib/utils';
import { Progress } from '@/ui/shadcn/progress';

export function IndeterminateProgress({
    className,
    ...props
}: Omit<React.ComponentProps<typeof Progress>, 'value'>) {
    return (
        <Progress
            value={null}
            className={cn('indeterminate-progress', className)}
            {...props}
        />
    );
}
