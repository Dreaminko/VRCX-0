import { ArrowDownIcon } from 'lucide-react';
import type { ReactNode } from 'react';

import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerContent,
    MessageScrollerItem,
    MessageScrollerProvider,
    MessageScrollerViewport
} from '@/ui/shadcn/message-scroller';

import type { UIMessage } from '../assistantTypes';
import { AssistantMessage } from './AssistantMessage';

interface AssistantTranscriptProps {
    sessionId: string | null;
    messages: readonly UIMessage[] | undefined;
    emptyState: ReactNode;
    scrollToLatestLabel: string;
    thinkingLabel: string;
}

export function AssistantTranscript({
    sessionId,
    messages,
    emptyState,
    scrollToLatestLabel,
    thinkingLabel
}: AssistantTranscriptProps) {
    return (
        <MessageScrollerProvider
            key={sessionId ?? 'new-session'}
            autoScroll
            defaultScrollPosition="last-anchor"
        >
            <MessageScroller>
                <MessageScrollerViewport>
                    <MessageScrollerContent className="gap-4 p-4">
                        {messages?.length ? (
                            messages.map((message) => (
                                <MessageScrollerItem
                                    key={message.id}
                                    messageId={message.id}
                                    scrollAnchor={message.role === 'user'}
                                >
                                    <AssistantMessage
                                        message={message}
                                        thinkingLabel={thinkingLabel}
                                    />
                                </MessageScrollerItem>
                            ))
                        ) : (
                            <MessageScrollerItem>
                                {emptyState}
                            </MessageScrollerItem>
                        )}
                    </MessageScrollerContent>
                </MessageScrollerViewport>
                <MessageScrollerButton title={scrollToLatestLabel}>
                    <ArrowDownIcon />
                    <span className="sr-only">{scrollToLatestLabel}</span>
                </MessageScrollerButton>
            </MessageScroller>
        </MessageScrollerProvider>
    );
}
