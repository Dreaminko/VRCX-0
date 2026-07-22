import { beforeEach, describe, expect, it } from 'vitest';

import { useAssistantChatStore } from './assistantChatStore';

describe('assistantChatStore', () => {
    beforeEach(() => {
        useAssistantChatStore.setState({
            messagesBySession: {},
            busySessions: {},
            sessions: []
        });
    });

    it('replaces streamed draft text with the canonical final answer', () => {
        const store = useAssistantChatStore.getState();
        store.applyDelta({
            sessionId: 'session-1',
            turnId: 'turn-1',
            text: '| 1 | [Friend Name 1] | [Time Minutes] |',
            replace: false
        });
        store.applyDelta({
            sessionId: 'session-1',
            turnId: 'turn-1',
            text: 'Alice has the most mutual connections.',
            replace: true
        });

        expect(
            useAssistantChatStore.getState().messagesBySession['session-1']
        ).toMatchObject([
            {
                text: 'Alice has the most mutual connections.',
                streaming: true
            }
        ]);
    });
});
