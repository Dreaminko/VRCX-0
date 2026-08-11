import { create } from 'zustand';

type VrchatConfigSnapshot = Record<string, unknown>;

interface VrchatConfigState {
    snapshot: VrchatConfigSnapshot | null;
    setSnapshot: (snapshot: VrchatConfigSnapshot) => void;
    reset: () => void;
}

export const useVrchatConfigStore = create<VrchatConfigState>((set) => ({
    snapshot: null,
    setSnapshot(snapshot) {
        set({ snapshot });
    },
    reset() {
        set({ snapshot: null });
    }
}));

export type { VrchatConfigSnapshot, VrchatConfigState };
