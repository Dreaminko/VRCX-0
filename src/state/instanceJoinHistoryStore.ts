import { create } from 'zustand';

import { instanceLocationKey } from '@/domain/presence/instancePresence';

interface InstanceJoinHistoryStoreState {
    joinedAtByLocation: Record<string, number>;
    setInstanceJoinHistory: (entries: Iterable<[unknown, unknown]>) => void;
    recordInstanceJoin: (location: unknown, joinedAt: unknown) => void;
    resetInstanceJoinHistory: () => void;
}

const initialState: Pick<InstanceJoinHistoryStoreState, 'joinedAtByLocation'> =
    {
        joinedAtByLocation: {}
    };

function epochMs(value: unknown): number {
    const parsed =
        typeof value === 'number' ? value : Date.parse(String(value ?? ''));
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 0;
}

export const useInstanceJoinHistoryStore =
    create<InstanceJoinHistoryStoreState>((set) => ({
        ...initialState,
        setInstanceJoinHistory(entries) {
            const joinedAtByLocation: Record<string, number> = {};
            for (const [location, joinedAt] of entries) {
                const key = instanceLocationKey(location);
                const epoch = epochMs(joinedAt);
                if (!key || !epoch) {
                    continue;
                }
                const existing = joinedAtByLocation[key];
                joinedAtByLocation[key] = existing
                    ? Math.min(existing, epoch)
                    : epoch;
            }
            set({ joinedAtByLocation });
        },
        recordInstanceJoin(location, joinedAt) {
            set((state) => {
                const key = instanceLocationKey(location);
                const epoch = epochMs(joinedAt);
                if (!key || !epoch) {
                    return state;
                }
                const existing = state.joinedAtByLocation[key];
                if (existing && existing <= epoch) {
                    return state;
                }
                return {
                    joinedAtByLocation: {
                        ...state.joinedAtByLocation,
                        [key]: epoch
                    }
                };
            });
        },
        resetInstanceJoinHistory() {
            set(initialState);
        }
    }));

export type { InstanceJoinHistoryStoreState };
