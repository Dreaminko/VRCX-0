import { create } from 'zustand';

type WorldCollectionImportState = {
    active: boolean;
    progress: number;
    total: number;
    start(total: number): void;
    setProgress(progress: number): void;
    finish(): void;
};

const idleState = {
    active: false,
    progress: 0,
    total: 0
};

export const useWorldCollectionImportStore = create<WorldCollectionImportState>(
    (set) => ({
        ...idleState,
        start(total) {
            set({ active: true, progress: 0, total });
        },
        setProgress(progress) {
            set({ progress });
        },
        finish() {
            set(idleState);
        }
    })
);
