import { CheckIcon, XIcon } from 'lucide-react';

import { cn } from '@/lib/utils';
import { Marker, MarkerContent, MarkerIcon } from '@/ui/shadcn/marker';
import { Spinner } from '@/ui/shadcn/spinner';

import type {
    ToolCallStatus as Status,
    UIToolCall
} from '@/domain/assistant/assistantTypes';

interface ToolCallStatusProps {
    toolCall: UIToolCall;
}

const STATUS_ICON = {
    pending: Spinner,
    done: CheckIcon,
    error: XIcon
} satisfies Record<Status, React.ComponentType>;

function formatToolName(name: string): string {
    const spaced = name.replace(/_/g, ' ').trim();
    return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

export function ToolCallStatus({ toolCall }: ToolCallStatusProps) {
    const StatusIcon = STATUS_ICON[toolCall.status];

    return (
        <Marker
            role={toolCall.status === 'pending' ? 'status' : undefined}
            className={cn(
                'w-fit',
                toolCall.status === 'error' && 'text-destructive'
            )}
            title={toolCall.summary || undefined}
        >
            <MarkerIcon>
                <StatusIcon />
            </MarkerIcon>
            <MarkerContent>{formatToolName(toolCall.name)}</MarkerContent>
        </Marker>
    );
}
