import { beforeEach, describe, expect, it } from 'vitest';

import { useAssistantChatStore } from './assistantChatStore';

describe('assistantChatStore', () => {
    beforeEach(() => {
        useAssistantChatStore.getState().resetAssistantChatState();
    });

    it('replaces streamed draft text with the canonical final answer', () => {
        const store = useAssistantChatStore.getState();
        store.applyDelta({
            ownerUserId: 'usr_self',
            sessionId: 'session-1',
            turnId: 'turn-1',
            text: '| 1 | [Friend Name 1] | [Time Minutes] |',
            replace: false
        });
        store.applyDelta({
            ownerUserId: 'usr_self',
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

    it('evicts completed session data when the dialog closes and keeps active turns', () => {
        useAssistantChatStore.setState({
            open: true,
            activeSessionId: 'session-completed',
            sessions: [
                {
                    id: 'session-completed',
                    title: 'Completed',
                    busy: false,
                    updatedAt: '2026-08-11T00:00:00Z'
                },
                {
                    id: 'session-running',
                    title: 'Running',
                    busy: true,
                    updatedAt: '2026-08-11T00:00:00Z'
                }
            ],
            messagesBySession: {
                'session-completed': [],
                'session-running': []
            },
            surfacedEntitiesBySession: {
                'session-completed': [],
                'session-running': []
            },
            entityPanelOpenBySession: {
                'session-completed': true,
                'session-running': true
            },
            busySessions: {
                'session-completed': false,
                'session-running': true
            }
        });

        useAssistantChatStore.getState().setOpen(false);

        expect(useAssistantChatStore.getState()).toMatchObject({
            open: false,
            activeSessionId: 'session-completed',
            messagesBySession: { 'session-running': [] },
            surfacedEntitiesBySession: { 'session-running': [] },
            entityPanelOpenBySession: { 'session-running': true },
            busySessions: { 'session-running': true }
        });
    });

    it('removes every mirror owned by a deleted session', () => {
        useAssistantChatStore.setState({
            activeSessionId: 'session-1',
            sessions: [
                {
                    id: 'session-1',
                    title: 'First',
                    busy: false,
                    updatedAt: '2026-08-11T00:00:00Z'
                },
                {
                    id: 'session-2',
                    title: 'Second',
                    busy: false,
                    updatedAt: '2026-08-11T00:00:00Z'
                }
            ],
            messagesBySession: {
                'session-1': [],
                'session-2': []
            },
            surfacedEntitiesBySession: {
                'session-1': [],
                'session-2': []
            },
            entityPanelOpenBySession: {
                'session-1': true,
                'session-2': false
            },
            busySessions: {
                'session-1': false,
                'session-2': false
            }
        });

        useAssistantChatStore.getState().removeSession('session-1');

        expect(useAssistantChatStore.getState()).toMatchObject({
            activeSessionId: null,
            sessions: [
                {
                    id: 'session-2'
                }
            ],
            messagesBySession: { 'session-2': [] },
            surfacedEntitiesBySession: { 'session-2': [] },
            entityPanelOpenBySession: { 'session-2': false },
            busySessions: { 'session-2': false }
        });
    });

    it('resets all account-scoped assistant state', () => {
        const authScopeVersion =
            useAssistantChatStore.getState().authScopeVersion;
        useAssistantChatStore.setState({
            open: true,
            activeSessionId: 'session-1',
            sessions: [
                {
                    id: 'session-1',
                    title: 'First',
                    busy: true,
                    updatedAt: '2026-08-11T00:00:00Z'
                }
            ],
            messagesBySession: { 'session-1': [] },
            surfacedEntitiesBySession: { 'session-1': [] },
            entityPanelOpenBySession: { 'session-1': true },
            busySessions: { 'session-1': true }
        });

        useAssistantChatStore.getState().resetAssistantChatState();

        expect(useAssistantChatStore.getState()).toMatchObject({
            open: false,
            authScopeVersion: authScopeVersion + 1,
            sessions: [],
            activeSessionId: null,
            messagesBySession: {},
            surfacedEntitiesBySession: {},
            entityPanelOpenBySession: {},
            busySessions: {}
        });
    });
});
