import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { BackgroundImageProjection } from '@/platform/tauri/bindings';

const mocks = vi.hoisted(() => ({
    appBackgroundImageStateGet: vi.fn(),
    appBackgroundImageConfigure: vi.fn(),
    appBackgroundImageRefresh: vi.fn(),
    appOpenBackgroundImageFilesSelectorDialog: vi.fn(),
    appOpenFolderSelectorDialog: vi.fn(),
    convertFileSrc: vi.fn(
        (path: string, protocol: string) => `${protocol}://localhost/${path}`
    ),
    disableCommunityThemesForBackgroundImage: vi.fn(),
    registerBackgroundImageAppearanceHandlers: vi.fn(),
    syncBackgroundImageAppearance: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appBackgroundImageStateGet: mocks.appBackgroundImageStateGet,
        appBackgroundImageConfigure: mocks.appBackgroundImageConfigure,
        appBackgroundImageRefresh: mocks.appBackgroundImageRefresh,
        appOpenBackgroundImageFilesSelectorDialog:
            mocks.appOpenBackgroundImageFilesSelectorDialog,
        appOpenFolderSelectorDialog: mocks.appOpenFolderSelectorDialog
    }
}));

vi.mock('@/platform/tauri/assets', () => ({
    convertFileSrc: mocks.convertFileSrc
}));

vi.mock('@/services/appearanceConflictCoordinator', () => ({
    disableCommunityThemesForBackgroundImage:
        mocks.disableCommunityThemesForBackgroundImage,
    registerBackgroundImageAppearanceHandlers:
        mocks.registerBackgroundImageAppearanceHandlers
}));

vi.mock('./appearanceService', () => ({
    syncBackgroundImageAppearance: mocks.syncBackgroundImageAppearance
}));

import { useBackgroundImageStore } from '@/state/backgroundImageStore';

import {
    applyBackgroundImageProjectionEvent,
    disableBackgroundImage,
    initializeBackgroundImage,
    setBackgroundImageMode
} from './backgroundImageService';

let nextRevision = 1;

function dailyProjection(
    overrides: Partial<BackgroundImageProjection> = {}
): BackgroundImageProjection {
    return {
        revision: nextRevision++,
        enabled: true,
        mode: 'daily',
        providerId: 'nasa-epic',
        customSource: null,
        snapshot: {
            mode: 'daily',
            providerId: 'nasa-epic',
            imageUrl: 'https://epic.gsfc.nasa.gov/a.jpg',
            title: 'Earth',
            author: 'NASA EPIC / DSCOVR',
            license: 'NASA media usage guidelines',
            source: 'NASA EPIC',
            resolvedAt: '2026-07-30T00:00:00.000Z',
            resolvedForKey: '2026-07-30'
        },
        error: null,
        ...overrides
    };
}

describe('backgroundImageService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.syncBackgroundImageAppearance.mockResolvedValue(undefined);
        mocks.disableCommunityThemesForBackgroundImage.mockResolvedValue(
            undefined
        );
        useBackgroundImageStore.getState().applyProjection({
            mode: 'off',
            enabled: false,
            providerId: 'nasa-epic',
            customSource: null,
            snapshot: null,
            error: null
        });
    });

    it('hydrates the store from the runtime projection on initialize', async () => {
        mocks.appBackgroundImageStateGet.mockResolvedValue(dailyProjection());

        await initializeBackgroundImage();

        const state = useBackgroundImageStore.getState();
        expect(state.enabled).toBe(true);
        expect(state.mode).toBe('daily');
        expect(state.snapshot?.imageUrl).toBe(
            'https://epic.gsfc.nasa.gov/a.jpg'
        );
        expect(mocks.syncBackgroundImageAppearance).toHaveBeenCalledWith(false);
    });

    it('materializes a local image URL for custom snapshots', async () => {
        mocks.appBackgroundImageStateGet.mockResolvedValue(
            dailyProjection({
                mode: 'custom',
                customSource: {
                    kind: 'files',
                    paths: ['C:\\img\\a.png'],
                    folderPath: '',
                    rotationInterval: 'daily'
                },
                snapshot: {
                    mode: 'custom',
                    sourceKind: 'files',
                    imageUrl: '',
                    imagePath: 'C:\\img\\a.png',
                    imageCount: 1,
                    title: 'a.png',
                    author: 'Custom image source',
                    license: 'Local file',
                    source: '1 selected image',
                    resolvedAt: '2026-07-30T00:00:00.000Z',
                    resolvedForKey: 'static'
                }
            })
        );

        await initializeBackgroundImage();

        expect(useBackgroundImageStore.getState().snapshot?.imageUrl).toBe(
            'vrcx-0-bg-img://localhost/C:\\img\\a.png?v=static'
        );
    });

    it('disables community themes before applying an enabling configure result', async () => {
        mocks.appBackgroundImageConfigure.mockResolvedValue(dailyProjection());

        await expect(setBackgroundImageMode('daily')).resolves.toBe(true);

        expect(mocks.appBackgroundImageConfigure).toHaveBeenCalledWith({
            kind: 'enableDaily',
            providerId: null
        });
        expect(
            mocks.disableCommunityThemesForBackgroundImage
        ).toHaveBeenCalledTimes(1);
        expect(useBackgroundImageStore.getState().enabled).toBe(true);
    });

    it('keeps the community theme untouched when disabling and records errors', async () => {
        mocks.appBackgroundImageConfigure.mockResolvedValue(
            dailyProjection({ enabled: false, mode: 'off', snapshot: null })
        );

        await disableBackgroundImage({ restoreAppTheme: false });

        expect(
            mocks.disableCommunityThemesForBackgroundImage
        ).not.toHaveBeenCalled();
        expect(mocks.syncBackgroundImageAppearance).toHaveBeenCalledWith(false);
        expect(useBackgroundImageStore.getState().enabled).toBe(false);

        mocks.appBackgroundImageConfigure.mockRejectedValue(
            new Error('configure failed')
        );
        await expect(disableBackgroundImage()).rejects.toThrow(
            'configure failed'
        );
        expect(useBackgroundImageStore.getState().error).toBe(
            'configure failed'
        );
    });

    it('applies pushed projections but skips revisions already applied', async () => {
        const projection = dailyProjection();
        mocks.appBackgroundImageStateGet.mockResolvedValue(projection);
        await initializeBackgroundImage();
        mocks.syncBackgroundImageAppearance.mockClear();

        applyBackgroundImageProjectionEvent({
            ...projection,
            snapshot: {
                ...projection.snapshot!,
                imageUrl: 'https://epic.gsfc.nasa.gov/echo.jpg'
            }
        });
        expect(mocks.syncBackgroundImageAppearance).not.toHaveBeenCalled();
        expect(useBackgroundImageStore.getState().snapshot?.imageUrl).toBe(
            'https://epic.gsfc.nasa.gov/a.jpg'
        );

        const rotated = dailyProjection({
            snapshot: {
                ...projection.snapshot!,
                imageUrl: 'https://epic.gsfc.nasa.gov/b.jpg'
            }
        });
        applyBackgroundImageProjectionEvent(rotated);
        expect(useBackgroundImageStore.getState().snapshot?.imageUrl).toBe(
            'https://epic.gsfc.nasa.gov/b.jpg'
        );
    });
});
