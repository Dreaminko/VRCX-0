// @vitest-environment jsdom

import { cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    appLlmEndpointDetectModels: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appLlmEndpointDetectModels: mocks.appLlmEndpointDetectModels
    }
}));

import { useAssistantChatStore } from '@/state/assistantChatStore';

import { useAssistantHealth } from './useAssistantHealth';

describe('useAssistantHealth', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        useAssistantChatStore.getState().setOpen(true);
        mocks.appLlmEndpointDetectModels.mockReturnValue(new Promise(() => {}));
    });

    afterEach(() => {
        cleanup();
        useAssistantChatStore.getState().setOpen(false);
    });

    it('immediately reports unconfigured when the selected endpoint disappears', () => {
        const initialProps: { endpointId: string | null } = {
            endpointId: 'endpoint-1'
        };
        const { result, rerender } = renderHook(
            ({ endpointId }: { endpointId: string | null }) =>
                useAssistantHealth(endpointId),
            { initialProps }
        );

        expect(result.current).toBe('checking');
        expect(mocks.appLlmEndpointDetectModels).toHaveBeenCalledOnce();

        rerender({ endpointId: null });

        expect(result.current).toBe('unconfigured');
    });
});
