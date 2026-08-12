import { useEffect } from 'react';

import { tauriClient } from '@/platform/tauri/client';
import {
    recordAssistantToolError,
    recordAssistantTurnError
} from '@/services/telemetry/telemetryAssistantHealth';
import { useAssistantChatStore } from '@/state/assistantChatStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import type {
    AssistantDeltaEvent,
    AssistantDoneEvent,
    AssistantErrorEvent,
    AssistantToolCallEvent,
    AssistantToolResultEvent,
    AssistantTurnEntitiesEvent
} from '@/domain/assistant/assistantTypes';

const EVENT_NAMES = [
    'assistantDelta',
    'assistantToolCall',
    'assistantToolResult',
    'assistantTurnEntities',
    'assistantDone',
    'assistantError'
] as const;

function isCurrentAccountEvent(event: {
    ownerUserId: string;
    sessionId: string;
}): boolean {
    if (
        !event.ownerUserId ||
        event.ownerUserId !== useRuntimeStore.getState().auth.currentUserId
    ) {
        return false;
    }
    const state = useAssistantChatStore.getState();
    return (
        state.open ||
        Boolean(state.busySessions[event.sessionId]) ||
        state.sessions.some(
            (session) => session.id === event.sessionId && session.busy
        )
    );
}

export function useAssistantEvents(): void {
    useEffect(() => {
        const store = useAssistantChatStore.getState();
        const unsubscribers: Array<() => void> = [];
        let active = true;

        // Coalesce per-token deltas into one store commit per animation frame.
        // A fast model streams 20-60 tokens/sec; without this each token would
        // trigger a full store update + markdown re-parse + re-render.
        const pendingDeltas = new Map<string, AssistantDeltaEvent>();
        const toolCallsById = new Map<
            string,
            Pick<AssistantToolCallEvent, 'name' | 'args'>
        >();
        let rafHandle = 0;
        const flushDeltas = () => {
            rafHandle = 0;
            for (const event of pendingDeltas.values()) {
                if (isCurrentAccountEvent(event)) {
                    store.applyDelta(event);
                }
            }
            pendingDeltas.clear();
        };
        const flushNow = () => {
            if (rafHandle) {
                cancelAnimationFrame(rafHandle);
            }
            flushDeltas();
        };
        const clearBufferedEvents = () => {
            if (rafHandle) {
                cancelAnimationFrame(rafHandle);
                rafHandle = 0;
            }
            pendingDeltas.clear();
            toolCallsById.clear();
        };
        const evictFinishedSessionIfClosed = (sessionId: string) => {
            const current = useAssistantChatStore.getState();
            if (!current.open) {
                current.evictSessionData(sessionId);
            }
        };

        const handlers: Record<string, (payload: unknown) => void> = {
            assistantDelta: (payload) => {
                const event = payload as AssistantDeltaEvent;
                if (!isCurrentAccountEvent(event)) {
                    return;
                }
                if (event.replace) {
                    flushNow();
                    store.applyDelta(event);
                    return;
                }
                const buffered = pendingDeltas.get(event.turnId);
                if (buffered) {
                    buffered.text += event.text;
                } else {
                    pendingDeltas.set(event.turnId, { ...event });
                }
                if (!rafHandle) {
                    rafHandle = requestAnimationFrame(flushDeltas);
                }
            },
            assistantToolCall: (payload) => {
                const event = payload as AssistantToolCallEvent;
                if (!isCurrentAccountEvent(event)) {
                    return;
                }
                flushNow();
                toolCallsById.set(event.toolCallId, {
                    name: event.name,
                    args: event.args
                });
                store.applyToolCall(event);
            },
            assistantToolResult: (payload) => {
                const event = payload as AssistantToolResultEvent;
                if (!isCurrentAccountEvent(event)) {
                    return;
                }
                flushNow();
                store.applyToolResult(event);
                if (!event.ok) {
                    const tool = toolCallsById.get(event.toolCallId);
                    recordAssistantToolError({
                        source: tool?.name,
                        args: tool?.args,
                        summary: event.summary
                    });
                }
                toolCallsById.delete(event.toolCallId);
            },
            assistantTurnEntities: (payload) => {
                const event = payload as AssistantTurnEntitiesEvent;
                if (isCurrentAccountEvent(event)) {
                    store.applyTurnEntities(event);
                }
            },
            assistantDone: (payload) => {
                const event = payload as AssistantDoneEvent;
                if (isCurrentAccountEvent(event)) {
                    flushNow();
                    store.applyDone(event);
                    evictFinishedSessionIfClosed(event.sessionId);
                }
            },
            assistantError: (payload) => {
                const event = payload as AssistantErrorEvent;
                if (!isCurrentAccountEvent(event)) {
                    return;
                }
                flushNow();
                store.applyError(event);
                recordAssistantTurnError(event.code, event.message);
                evictFinishedSessionIfClosed(event.sessionId);
            }
        };

        const unsubscribeAuth = useRuntimeStore.subscribe(
            (state, previousState) => {
                if (
                    state.auth.currentUserId ===
                    previousState.auth.currentUserId
                ) {
                    return;
                }
                clearBufferedEvents();
            }
        );
        const unsubscribeAssistantScope = useAssistantChatStore.subscribe(
            (state, previousState) => {
                if (state.authScopeVersion !== previousState.authScopeVersion) {
                    clearBufferedEvents();
                }
            }
        );

        for (const name of EVENT_NAMES) {
            tauriClient.events
                .subscribe<unknown>(name, handlers[name])
                .then((unsubscribe) => {
                    if (active) {
                        unsubscribers.push(unsubscribe);
                    } else {
                        unsubscribe();
                    }
                })
                .catch(() => {});
        }

        return () => {
            active = false;
            if (rafHandle) {
                cancelAnimationFrame(rafHandle);
            }
            for (const unsubscribe of unsubscribers) {
                unsubscribe();
            }
            unsubscribeAuth();
            unsubscribeAssistantScope();
            toolCallsById.clear();
        };
    }, []);
}
