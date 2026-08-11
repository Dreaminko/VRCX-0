import { create } from 'zustand';

import {
    commands,
    type LlmEndpointDetectModelsInput,
    type LlmEndpointDetectModelsResult,
    type LlmEndpointDto,
    type LlmEndpointUpsertInput
} from '@/platform/tauri/bindings';
import { useRuntimeStore } from '@/state/runtimeStore';

export function openLlmEndpointsManager(): void {
    useRuntimeStore.getState().setSystemHostOpen('llmEndpointsOpen', true);
}

type LlmEndpointsStoreState = {
    endpoints: LlmEndpointDto[];
    loading: boolean;
    error: string | null;
    load: () => Promise<LlmEndpointDto[]>;
    upsert: (input: LlmEndpointUpsertInput) => Promise<LlmEndpointDto>;
    deleteEndpoint: (id: string) => Promise<void>;
    detectModels: (
        input: LlmEndpointDetectModelsInput
    ) => Promise<LlmEndpointDetectModelsResult>;
};

export function mergeModels(...lists: string[][]): string[] {
    const models = lists
        .flat()
        .map((model) => model.trim())
        .filter(Boolean);
    models.sort();
    return [...new Set(models)];
}

function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}

export const useLlmEndpointsStore = create<LlmEndpointsStoreState>((set) => {
    let pendingOperations = 0;
    let endpointRevision = 0;

    function beginOperation(): void {
        pendingOperations += 1;
        set({ loading: true, error: null });
    }

    function finishOperation(): void {
        pendingOperations -= 1;
        set({ loading: pendingOperations > 0 });
    }

    return {
        endpoints: [],
        loading: false,
        error: null,
        async load() {
            const expectedRevision = endpointRevision;
            beginOperation();
            try {
                const endpoints = await commands.appLlmEndpointList();
                if (expectedRevision === endpointRevision) {
                    set({ endpoints });
                }
                return endpoints;
            } catch (error) {
                set({ error: errorMessage(error) });
                throw error;
            } finally {
                finishOperation();
            }
        },
        async upsert(input) {
            beginOperation();
            try {
                const saved = await commands.appLlmEndpointUpsert(input);
                endpointRevision += 1;
                set((state) => {
                    const exists = state.endpoints.some(
                        (endpoint) => endpoint.id === saved.id
                    );
                    return {
                        endpoints: exists
                            ? state.endpoints.map((endpoint) =>
                                  endpoint.id === saved.id ? saved : endpoint
                              )
                            : [...state.endpoints, saved]
                    };
                });
                return saved;
            } catch (error) {
                set({ error: errorMessage(error) });
                throw error;
            } finally {
                finishOperation();
            }
        },
        async deleteEndpoint(id) {
            beginOperation();
            try {
                await commands.appLlmEndpointDelete(id);
                endpointRevision += 1;
                set((state) => ({
                    endpoints: state.endpoints.filter(
                        (endpoint) => endpoint.id !== id
                    )
                }));
            } catch (error) {
                set({ error: errorMessage(error) });
                throw error;
            } finally {
                finishOperation();
            }
        },
        async detectModels(input) {
            const expectedRevision = endpointRevision;
            beginOperation();
            try {
                const result = await commands.appLlmEndpointDetectModels(input);
                if (input.id && input.persist) {
                    const endpoints = await commands.appLlmEndpointList();
                    if (expectedRevision === endpointRevision) {
                        endpointRevision += 1;
                        set({ endpoints });
                    }
                }
                return result;
            } catch (error) {
                set({ error: errorMessage(error) });
                throw error;
            } finally {
                finishOperation();
            }
        }
    };
});
