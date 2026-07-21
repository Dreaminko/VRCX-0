import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getShellState: vi.fn(),
    normalizeZoomLevel: vi.fn((value: unknown) => Number(value)),
    setZoomLevelPreference: vi.fn()
}));

vi.mock('@/state/shellStore', () => ({
    useShellStore: { getState: mocks.getShellState }
}));

vi.mock('./preferencesService', () => ({
    setZoomLevelPreference: mocks.setZoomLevelPreference
}));

vi.mock('./themeService', () => ({
    normalizeZoomLevel: mocks.normalizeZoomLevel
}));

function deferred() {
    let resolve!: () => void;
    let reject!: (error: unknown) => void;
    const promise = new Promise<void>((resolvePromise, rejectPromise) => {
        resolve = resolvePromise;
        reject = rejectPromise;
    });
    return { promise, reject, resolve };
}

async function loadService() {
    vi.resetModules();
    return import('./zoomPreferenceService');
}

describe('zoomPreferenceService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.getShellState.mockReturnValue({ zoomLevel: 100 });
        mocks.normalizeZoomLevel.mockImplementation((value: unknown) =>
            Math.min(200, Math.max(50, Math.trunc(Number(value))))
        );
    });

    it('coalesces queued writes while preserving the latest step target', async () => {
        const writes = [deferred(), deferred(), deferred()];
        let currentZoom = 100;
        mocks.getShellState.mockImplementation(() => ({
            zoomLevel: currentZoom
        }));
        mocks.setZoomLevelPreference.mockImplementation(
            async (value: number) => {
                const write =
                    writes[mocks.setZoomLevelPreference.mock.calls.length - 1];
                await write.promise;
                currentZoom = value;
            }
        );
        const { stepQueuedZoomLevelPreference } = await loadService();

        expect(stepQueuedZoomLevelPreference(10)).toBe(110);
        expect(stepQueuedZoomLevelPreference(10)).toBe(120);
        expect(stepQueuedZoomLevelPreference(10)).toBe(130);

        writes[0].resolve();
        await vi.waitFor(() => {
            expect(mocks.setZoomLevelPreference).toHaveBeenNthCalledWith(
                2,
                130
            );
        });
        expect(mocks.setZoomLevelPreference).not.toHaveBeenCalledWith(120);

        expect(stepQueuedZoomLevelPreference(10)).toBe(140);
        writes[1].resolve();
        await vi.waitFor(() => {
            expect(mocks.setZoomLevelPreference).toHaveBeenNthCalledWith(
                3,
                140
            );
        });

        writes[2].resolve();
        await vi.waitFor(() => expect(currentZoom).toBe(140));
    });

    it('uses the current store value after a failed latest write', async () => {
        const error = new Error('persist failed');
        const onError = vi.fn();
        mocks.setZoomLevelPreference.mockRejectedValueOnce(error);
        const { queueZoomLevelPreference, stepQueuedZoomLevelPreference } =
            await loadService();

        queueZoomLevelPreference(150, { onError });
        await vi.waitFor(() => expect(onError).toHaveBeenCalledWith(error));

        expect(stepQueuedZoomLevelPreference(10)).toBe(110);
        await vi.waitFor(() => {
            expect(mocks.setZoomLevelPreference).toHaveBeenLastCalledWith(110);
        });
    });

    it('only accepts external synchronization while the queue is idle', async () => {
        const write = deferred();
        mocks.setZoomLevelPreference.mockReturnValue(write.promise);
        const { stepQueuedZoomLevelPreference, syncQueuedZoomLevel } =
            await loadService();

        syncQueuedZoomLevel(125);
        expect(stepQueuedZoomLevelPreference(5)).toBe(130);
        syncQueuedZoomLevel(80);
        expect(stepQueuedZoomLevelPreference(5)).toBe(135);

        write.resolve();
    });
});
