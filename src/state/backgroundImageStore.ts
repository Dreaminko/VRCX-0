import { create } from 'zustand';

import type {
    BackgroundImageCustomSource,
    BackgroundImageMode,
    BackgroundImageProviderId,
    BackgroundImageSnapshot
} from '@/platform/tauri/bindings';

interface BackgroundImageStore {
    mode: BackgroundImageMode;
    enabled: boolean;
    providerId: BackgroundImageProviderId;
    customSource: BackgroundImageCustomSource | null;
    snapshot: BackgroundImageSnapshot | null;
    loading: boolean;
    error: string | null;
    applyProjection(options: {
        mode: BackgroundImageMode;
        enabled: boolean;
        providerId: BackgroundImageProviderId;
        customSource: BackgroundImageCustomSource | null;
        snapshot: BackgroundImageSnapshot | null;
        error: string | null;
    }): void;
    setLoading(loading: boolean): void;
    setError(error: string | null): void;
}

export const useBackgroundImageStore = create<BackgroundImageStore>((set) => ({
    mode: 'off',
    enabled: false,
    providerId: 'nasa-epic',
    customSource: null,
    snapshot: null,
    loading: false,
    error: null,
    applyProjection(options) {
        set(options);
    },
    setLoading(loading) {
        set({ loading: Boolean(loading) });
    },
    setError(error) {
        set({ error });
    }
}));
