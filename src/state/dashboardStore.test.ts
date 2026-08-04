import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getDashboards: vi.fn(),
    saveDashboards: vi.fn()
}));

vi.mock('@/repositories/dashboardRepository', async (importOriginal) => {
    const actual =
        await importOriginal<
            typeof import('@/repositories/dashboardRepository')
        >();
    return {
        ...actual,
        default: {
            ...actual.default,
            getDashboards: mocks.getDashboards,
            saveDashboards: mocks.saveDashboards
        }
    };
});

import { useDashboardStore } from './dashboardStore';

type Deferred<T> = {
    promise: Promise<T>;
    resolve: (value: T) => void;
};

function deferred<T>(): Deferred<T> {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>((promiseResolve) => {
        resolve = promiseResolve;
    });
    return { promise, resolve };
}

const existingDashboard = {
    id: 'dashboard-main',
    name: 'Main',
    icon: 'lucide:LayoutDashboard',
    rows: []
};

describe('dashboardStore persistence ownership', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        useDashboardStore.getState().resetDashboardState();
    });

    it('coalesces concurrent initial loads through one repository request', async () => {
        const load = deferred<(typeof existingDashboard)[]>();
        mocks.getDashboards.mockReturnValue(load.promise);

        const first = useDashboardStore.getState().ensureLoaded();
        const second = useDashboardStore.getState().ensureLoaded();

        expect(mocks.getDashboards).toHaveBeenCalledTimes(1);
        expect(useDashboardStore.getState().loadStatus).toBe('running');

        load.resolve([existingDashboard]);

        await expect(first).resolves.toEqual([existingDashboard]);
        await expect(second).resolves.toEqual([existingDashboard]);
        expect(useDashboardStore.getState()).toMatchObject({
            dashboards: [existingDashboard],
            loaded: true,
            loadStatus: 'ready',
            detail: ''
        });
    });

    it('keeps the previous mirror until a dashboard update is persisted', async () => {
        useDashboardStore.setState({
            dashboards: [existingDashboard],
            loaded: true,
            loadStatus: 'ready',
            detail: ''
        });
        const save = deferred<(typeof existingDashboard)[]>();
        mocks.saveDashboards.mockReturnValue(save.promise);

        const update = useDashboardStore
            .getState()
            .updateDashboard(existingDashboard.id, { name: 'Renamed' });

        expect(useDashboardStore.getState().dashboards).toEqual([
            existingDashboard
        ]);
        await vi.waitFor(() =>
            expect(mocks.saveDashboards).toHaveBeenCalledWith([
                { ...existingDashboard, name: 'Renamed' }
            ])
        );

        const persisted = [{ ...existingDashboard, name: 'Renamed' }];
        save.resolve(persisted);

        await expect(update).resolves.toEqual(persisted[0]);
        expect(useDashboardStore.getState().dashboards).toEqual(persisted);
    });

    it('retries an initial load after a transient repository failure', async () => {
        mocks.getDashboards
            .mockRejectedValueOnce(new Error('storage unavailable'))
            .mockResolvedValueOnce([existingDashboard]);

        await expect(
            useDashboardStore.getState().ensureLoaded()
        ).rejects.toThrow('storage unavailable');
        expect(useDashboardStore.getState()).toMatchObject({
            dashboards: [],
            loaded: false,
            loadStatus: 'error',
            detail: 'storage unavailable'
        });

        await expect(
            useDashboardStore.getState().ensureLoaded()
        ).resolves.toEqual([existingDashboard]);
        expect(mocks.getDashboards).toHaveBeenCalledTimes(2);
        expect(useDashboardStore.getState()).toMatchObject({
            dashboards: [existingDashboard],
            loaded: true,
            loadStatus: 'ready',
            detail: ''
        });
    });

    it('clears the editing target only after its dashboard is deleted', async () => {
        useDashboardStore.setState({
            dashboards: [existingDashboard],
            loaded: true,
            loadStatus: 'ready',
            editingDashboardId: existingDashboard.id
        });
        mocks.saveDashboards.mockResolvedValue([]);

        await useDashboardStore
            .getState()
            .deleteDashboard(existingDashboard.id);

        expect(mocks.saveDashboards).toHaveBeenCalledWith([]);
        expect(useDashboardStore.getState()).toMatchObject({
            dashboards: [],
            editingDashboardId: null
        });
    });
});
