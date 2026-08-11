// @vitest-environment jsdom

import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { ScreenshotLibraryScanStatus } from '@/platform/tauri/bindings';

type ScanStatusListener = (status: ScreenshotLibraryScanStatus) => void;

const mocks = vi.hoisted(() => ({
    getCurrentStatus:
        vi.fn<() => Promise<ScreenshotLibraryScanStatus | null>>(),
    getWorldScreenshots: vi.fn<() => Promise<unknown>>(),
    startScan:
        vi.fn<(force?: boolean) => Promise<ScreenshotLibraryScanStatus | null>>(),
    subscribe: vi.fn<(listener: ScanStatusListener) => () => void>(),
    unsubscribe: vi.fn()
}));

vi.mock('@/repositories/mediaRepository', () => ({
    default: {
        getWorldScreenshots: mocks.getWorldScreenshots
    }
}));

vi.mock('@/services/screenshotLibraryScanService', () => ({
    getCurrentScreenshotLibraryScanStatus: mocks.getCurrentStatus,
    startScreenshotLibraryScan: mocks.startScan,
    subscribeScreenshotLibraryScanStatus: mocks.subscribe
}));

import { useWorldDialogScreenshots } from './useWorldDialogScreenshots';

function scanStatus(
    overrides: Partial<ScreenshotLibraryScanStatus> = {}
): ScreenshotLibraryScanStatus {
    return {
        running: false,
        scanned: 0,
        indexed: 0,
        changed: 0,
        skipped: 0,
        deleted: 0,
        error: null,
        lastScanAt: null,
        ...overrides
    };
}

function screenshot() {
    return {
        path: 'C:\\Screenshots\\VRChat_2026-08-11.png',
        folderPath: 'C:\\Screenshots',
        fileName: 'VRChat_2026-08-11.png',
        sizeBytes: 128,
        modifiedAt: 1,
        createdAt: 1,
        width: 1920,
        height: 1080,
        worldId: 'wrld_target',
        worldName: 'Target World',
        capturedAt: '2026-08-11T00:00:00Z',
        metadata: {
            application: 'VRChat',
            version: 1,
            author: { id: 'usr_author' },
            world: {
                id: 'wrld_target',
                instanceId: '12345'
            },
            players: [],
            sourceFile: 'VRChat_2026-08-11.png'
        },
        error: null
    };
}

describe('useWorldDialogScreenshots', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.subscribe.mockReturnValue(mocks.unsubscribe);
        mocks.getCurrentStatus.mockResolvedValue(scanStatus());
        mocks.startScan.mockResolvedValue(scanStatus());
        mocks.getWorldScreenshots.mockResolvedValue([screenshot()]);
    });

    it('loads screenshots only after the screenshots tab scan completes', async () => {
        const { result, rerender } = renderHook(
            ({ activeTab }) =>
                useWorldDialogScreenshots({
                    activeTab,
                    loadFailedMessage: 'load failed',
                    openNonce: 0,
                    worldId: 'wrld_target'
                }),
            { initialProps: { activeTab: 'instances' } }
        );

        expect(result.current.status).toBe('idle');
        expect(mocks.subscribe).not.toHaveBeenCalled();

        rerender({ activeTab: 'screenshots' });

        await waitFor(() => expect(result.current.status).toBe('ready'));
        expect(mocks.startScan).toHaveBeenCalledWith(false);
        expect(mocks.getWorldScreenshots).toHaveBeenCalledWith('wrld_target');
        expect(result.current.screenshots).toEqual([screenshot()]);
        expect(result.current.error).toBe('');
    });

    it('requests a forced scan when the user refreshes', async () => {
        const { result } = renderHook(() =>
            useWorldDialogScreenshots({
                activeTab: 'screenshots',
                loadFailedMessage: 'load failed',
                openNonce: 0,
                worldId: 'wrld_target'
            })
        );
        await waitFor(() => expect(result.current.status).toBe('ready'));
        mocks.startScan.mockClear();

        act(() => result.current.refresh());

        await waitFor(() =>
            expect(mocks.startScan).toHaveBeenCalledWith(true)
        );
    });

    it('surfaces a completed scan error when no screenshots are available', async () => {
        mocks.startScan.mockResolvedValue(
            scanStatus({ error: 'scan failed' })
        );
        mocks.getWorldScreenshots.mockResolvedValue([]);

        const { result } = renderHook(() =>
            useWorldDialogScreenshots({
                activeTab: 'screenshots',
                loadFailedMessage: 'load failed',
                openNonce: 0,
                worldId: 'wrld_target'
            })
        );

        await waitFor(() => expect(result.current.status).toBe('error'));
        expect(result.current.error).toBe('scan failed');
        expect(result.current.screenshots).toEqual([]);
    });

    it('cancels initialization after the owner unmounts', async () => {
        let resolveStatus!: (status: ScreenshotLibraryScanStatus) => void;
        mocks.getCurrentStatus.mockReturnValue(
            new Promise((resolve) => {
                resolveStatus = resolve;
            })
        );
        const { unmount } = renderHook(() =>
            useWorldDialogScreenshots({
                activeTab: 'screenshots',
                loadFailedMessage: 'load failed',
                openNonce: 0,
                worldId: 'wrld_target'
            })
        );

        unmount();
        await act(async () => resolveStatus(scanStatus()));

        expect(mocks.unsubscribe).toHaveBeenCalledOnce();
        expect(mocks.startScan).not.toHaveBeenCalled();
        expect(mocks.getWorldScreenshots).not.toHaveBeenCalled();
    });
});
