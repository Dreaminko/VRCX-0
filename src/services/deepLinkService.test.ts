import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    appDrainPendingDeepLinks:
        vi.fn<
            () => Promise<import('@/platform/tauri/bindings').DeepLinkAction[]>
        >(),
    eventHandler: null as ((payload: unknown) => void) | null,
    openAlert: vi.fn(),
    openWorldDialog: vi.fn(),
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

vi.mock('@/services/dialogService', () => ({
    openWorldDialog: mocks.openWorldDialog
}));

vi.mock('@/state/modalStore', () => ({
    useModalStore: {
        getState: () => ({
            openAlert: mocks.openAlert
        })
    }
}));

vi.mock('./i18nService', () => ({
    default: {
        t: (key: string, params?: Record<string, unknown>) =>
            params?.collectionId ? `${key}:${params.collectionId}` : key
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

        mocks.eventHandler?.({});

        await vi.waitFor(() => {
            expect(mocks.openAlert).toHaveBeenCalledWith(
                expect.objectContaining({
                    description:
                        'deep_link.import_collection.description:AbC123z'
                })
            );
        });
    });

    it('opens worlds and collection placeholders from actions', () => {
        handleDeepLinkAction({ type: 'openWorld', worldId: WORLD_ID });
        handleDeepLinkAction({
            type: 'importCollection',
            collectionId: 'Z9xY12'
        });

        expect(mocks.openWorldDialog).toHaveBeenCalledWith({
            worldId: WORLD_ID
        });
        expect(mocks.openAlert).toHaveBeenCalledWith(
            expect.objectContaining({
                title: 'deep_link.import_collection.title'
            })
        );
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
        expect(mocks.openAlert).not.toHaveBeenCalled();
    });
});
