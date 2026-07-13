import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    appDrainPendingDeepLinks:
        vi.fn<
            () => Promise<import('@/platform/tauri/bindings').DeepLinkAction[]>
        >(),
    eventHandler: null as ((payload: unknown) => void) | null,
    prompt: vi.fn(),
    openWorldDialog: vi.fn(),
    previewSharedCollection: vi.fn(),
    importWorldIdsToLocalGroup: vi.fn(),
    toastSuccess: vi.fn(),
    toastError: vi.fn(),
    unsubscribe: vi.fn(),
    subscribe:
        vi.fn<
            (
                name: string,
                handler: (payload: unknown) => void
            ) => Promise<() => void>
        >()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appDrainPendingDeepLinks: mocks.appDrainPendingDeepLinks
    }
}));

vi.mock('@/platform/tauri/client', () => ({
    tauriClient: {
        events: {
            subscribe: mocks.subscribe
        }
    }
}));

vi.mock('@/repositories/shareCollectionRepository', () => ({
    default: {
        previewSharedCollection: mocks.previewSharedCollection
    }
}));

vi.mock('@/services/dialogService', () => ({
    openWorldDialog: mocks.openWorldDialog
}));

vi.mock('@/services/favoriteImportService', () => ({
    importWorldIdsToLocalGroup: mocks.importWorldIdsToLocalGroup
}));

vi.mock('@/state/modalStore', () => ({
    useModalStore: {
        getState: () => ({
            prompt: mocks.prompt
        })
    }
}));

vi.mock('sonner', () => ({
    toast: {
        success: mocks.toastSuccess,
        error: mocks.toastError
    }
}));

vi.mock('./i18nService', () => ({
    default: {
        t: (key: string, params?: Record<string, unknown>) =>
            params ? `${key}:${JSON.stringify(params)}` : key
    }
}));

import {
    bindDeepLinkEvents,
    drainPendingDeepLinks,
    handleDeepLinkAction
} from './deepLinkService';

const WORLD_ID = 'wrld_12345678-1234-1234-1234-1234567890ab';

describe('deepLinkService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.eventHandler = null;
        mocks.appDrainPendingDeepLinks.mockResolvedValue([]);
        mocks.subscribe.mockImplementation(async (name, handler) => {
            expect(name).toBe('deepLinkArrived');
            mocks.eventHandler = handler;
            return mocks.unsubscribe;
        });
    });

    it('subscribes without draining queued links during binding', async () => {
        const unbind = await bindDeepLinkEvents();

        expect(mocks.appDrainPendingDeepLinks).not.toHaveBeenCalled();
        unbind();
        expect(mocks.unsubscribe).toHaveBeenCalledTimes(1);
    });

    it('drains queued links when the wake event arrives', async () => {
        await bindDeepLinkEvents();
        expect(mocks.appDrainPendingDeepLinks).not.toHaveBeenCalled();
        mocks.appDrainPendingDeepLinks.mockResolvedValueOnce([
            { type: 'importCollection', collectionId: 'AbC123z' }
        ]);
        mocks.previewSharedCollection.mockResolvedValueOnce({
            title: 'Scenic picks',
            worldIds: [WORLD_ID, WORLD_ID.replace(/ab$/, 'ac')]
        });
        mocks.prompt.mockResolvedValueOnce({ ok: false, reason: 'cancel' });

        mocks.eventHandler?.({});

        await vi.waitFor(() => {
            expect(mocks.previewSharedCollection).toHaveBeenCalledWith(
                'AbC123z'
            );
        });
        await vi.waitFor(() => {
            expect(mocks.prompt).toHaveBeenCalled();
        });
        expect(mocks.importWorldIdsToLocalGroup).not.toHaveBeenCalled();
    });

    it('imports the collection after confirmation and shows a success toast', () => {
        const secondWorldId = WORLD_ID.replace(/ab$/, 'ac');
        mocks.previewSharedCollection.mockResolvedValueOnce({
            title: 'Scenic picks',
            worldIds: [WORLD_ID, secondWorldId]
        });
        mocks.prompt.mockResolvedValueOnce({
            ok: true,
            reason: 'ok',
            value: ' My local worlds '
        });
        mocks.importWorldIdsToLocalGroup.mockImplementationOnce(
            async ({ onProgress }) => {
                onProgress?.(2, 2);
                return {
                    importedCount: 1,
                    failedCount: 1,
                    totalCount: 2
                };
            }
        );

        handleDeepLinkAction({
            type: 'importCollection',
            collectionId: 'Z9xY12'
        });

        return vi.waitFor(() => {
            expect(mocks.importWorldIdsToLocalGroup).toHaveBeenCalledWith({
                worldIds: [WORLD_ID, secondWorldId],
                groupName: 'My local worlds',
                onProgress: expect.any(Function)
            });
            expect(mocks.toastSuccess).toHaveBeenCalled();
            expect(mocks.toastError).toHaveBeenCalledWith(
                'deep_link.import_collection.toast.import_partial_failed:{"count":1}'
            );
        });
    });

    it('serializes collection imports so their progress cannot overlap', async () => {
        let releaseFirstImport: () => void = () => {};
        const firstImportGate = new Promise<void>((resolve) => {
            releaseFirstImport = resolve;
        });
        mocks.previewSharedCollection
            .mockResolvedValueOnce({
                title: 'First collection',
                worldIds: [WORLD_ID]
            })
            .mockResolvedValueOnce({
                title: 'Second collection',
                worldIds: [WORLD_ID.replace(/ab$/, 'ac')]
            });
        mocks.prompt
            .mockResolvedValueOnce({
                ok: true,
                reason: 'ok',
                value: 'First local group'
            })
            .mockResolvedValueOnce({
                ok: true,
                reason: 'ok',
                value: 'Second local group'
            });
        mocks.importWorldIdsToLocalGroup
            .mockImplementationOnce(async () => {
                await firstImportGate;
                return {
                    importedCount: 1,
                    failedCount: 0,
                    totalCount: 1
                };
            })
            .mockResolvedValueOnce({
                importedCount: 1,
                failedCount: 0,
                totalCount: 1
            });

        handleDeepLinkAction({
            type: 'importCollection',
            collectionId: 'First12'
        });
        handleDeepLinkAction({
            type: 'importCollection',
            collectionId: 'Second2'
        });

        await vi.waitFor(() => {
            expect(mocks.importWorldIdsToLocalGroup).toHaveBeenCalledTimes(1);
        });
        expect(mocks.previewSharedCollection).toHaveBeenCalledTimes(1);

        releaseFirstImport();

        await vi.waitFor(() => {
            expect(mocks.importWorldIdsToLocalGroup).toHaveBeenCalledTimes(2);
        });
        expect(mocks.previewSharedCollection).toHaveBeenNthCalledWith(
            2,
            'Second2'
        );
    });

    it('opens worlds from actions', () => {
        handleDeepLinkAction({ type: 'openWorld', worldId: WORLD_ID });

        expect(mocks.openWorldDialog).toHaveBeenCalledWith({
            worldId: WORLD_ID
        });
    });

    it('shows a toast when a shared collection has no importable worlds', () => {
        mocks.previewSharedCollection.mockResolvedValueOnce({
            title: 'Empty',
            worldIds: []
        });

        handleDeepLinkAction({
            type: 'importCollection',
            collectionId: 'EmptyId'
        });

        return vi.waitFor(() => {
            expect(mocks.toastError).toHaveBeenCalled();
            expect(mocks.prompt).not.toHaveBeenCalled();
        });
    });

    it('ignores malformed action payloads defensively', async () => {
        handleDeepLinkAction({ type: 'openWorld', worldId: 'bad' });
        handleDeepLinkAction({
            type: 'importCollection',
            collectionId: 'bad/value'
        });
        mocks.appDrainPendingDeepLinks.mockRejectedValueOnce(
            new Error('drain failed')
        );

        await drainPendingDeepLinks();

        expect(mocks.openWorldDialog).not.toHaveBeenCalled();
        expect(mocks.previewSharedCollection).not.toHaveBeenCalled();
    });
});
