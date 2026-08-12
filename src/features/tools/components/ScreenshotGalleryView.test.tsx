// @vitest-environment jsdom

import {
    cleanup,
    fireEvent,
    render,
    screen,
    waitFor
} from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    scrollIntoView: vi.fn()
}));

vi.mock('react-i18next', async (importOriginal) => ({
    ...(await importOriginal<typeof import('react-i18next')>()),
    useTranslation: () => ({
        t: (key: string, values?: { count?: number }) =>
            values?.count === undefined ? key : `${key}:${values.count}`
    })
}));

vi.mock('../useScreenshotGalleryGrid', () => ({
    useScreenshotGalleryGrid: () => ({
        gridColumnCount: 1,
        gridGap: 0,
        gridMinWidth: 0,
        totalHeight: 0,
        viewportRef: { current: null },
        visibleRows: []
    })
}));

vi.mock('./ScreenshotThumbnailGrid', () => ({
    ScreenshotThumbnailCard: () => null,
    useScreenshotThumbnailTitleMap: () => new Map()
}));

import { ScreenshotGalleryView } from './ScreenshotGalleryView';

const folderTree = {
    rootPath: 'C:\\VRChat',
    folders: [
        {
            path: 'C:\\VRChat',
            parentPath: null,
            name: 'VRChat',
            imageCount: 0,
            totalImageCount: 61,
            latestModifiedAt: 2
        },
        {
            path: 'C:\\VRChat\\2024-05',
            parentPath: 'C:\\VRChat',
            name: '2024-05',
            imageCount: 55,
            totalImageCount: 55,
            latestModifiedAt: 1
        },
        {
            path: 'C:\\VRChat\\2026-07',
            parentPath: 'C:\\VRChat',
            name: '2026-07',
            imageCount: 6,
            totalImageCount: 6,
            latestModifiedAt: 2
        }
    ]
};

describe('ScreenshotGalleryView folder tree', () => {
    beforeEach(() => {
        mocks.scrollIntoView.mockReset();
        HTMLElement.prototype.scrollIntoView = mocks.scrollIntoView;
    });

    afterEach(cleanup);

    it('uses one folder row per node and reveals the selected folder', async () => {
        const onSelectFolder = vi.fn();
        const { container } = render(
            <ScreenshotGalleryView
                folderTree={folderTree}
                images={[]}
                isImagesLoading={false}
                isTreeLoading={false}
                error=""
                scanStatus={null}
                selectedFolder={folderTree.folders[2].path}
                onOpenImage={() => undefined}
                onRefresh={() => undefined}
                onSelectFolder={onSelectFolder}
                onScrollPositionChange={() => undefined}
                restoreScrollTop={0}
            />
        );

        await waitFor(() => {
            expect(
                container.querySelectorAll(
                    'aside [data-slot="card-content"] button'
                )
            ).toHaveLength(3);
        });
        const selectedFolder = screen.getByRole('button', {
            name: '2026-07'
        });
        expect(selectedFolder.getAttribute('aria-current')).toBe('location');
        expect(mocks.scrollIntoView).toHaveBeenCalledWith({
            block: 'nearest',
            inline: 'nearest'
        });

        fireEvent.click(screen.getByRole('button', { name: '2024-05' }));
        expect(onSelectFolder).toHaveBeenCalledWith(folderTree.folders[1].path);
    });
});
