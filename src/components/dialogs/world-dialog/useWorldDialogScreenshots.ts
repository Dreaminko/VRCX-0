import { useEffect, useRef, useState } from 'react';

import mediaRepository from '@/repositories/mediaRepository';
import {
    getCurrentScreenshotLibraryScanStatus,
    startScreenshotLibraryScan,
    subscribeScreenshotLibraryScanStatus
} from '@/services/screenshotLibraryScanService';

export type WorldWorldScreenshots = Array<{
    path: string;
    folderPath: string;
    fileName: string;
    sizeBytes: number;
    modifiedAt: number;
    createdAt: number;
    width: number;
    height: number;
    worldId: string;
    worldName: string | null;
    capturedAt: string | null;
    metadata: {
        application: string;
        version: number;
        author: {
            id: string;
            displayName?: string;
        };
        world: {
            id: string;
            name?: string;
            instanceId: string;
        };
        players: Array<{
            id: string;
            displayName: string;
        }>;
        sourceFile: string;
        timestamp?: string;
    };
    error: string | null;
}>;

type ScreenshotScanStatus = Awaited<
    ReturnType<typeof mediaRepository.getScreenshotLibraryStatus>
>;

type WorldDialogScreenshotsInput = {
    activeTab: string;
    loadFailedMessage: string;
    openNonce: number;
    worldId: string;
};

function useWorldDialogScreenshots({
    activeTab,
    loadFailedMessage,
    openNonce,
    worldId
}: WorldDialogScreenshotsInput) {
    const [screenshots, setScreenshots] = useState<WorldWorldScreenshots>([]);
    const [status, setStatus] = useState('idle');
    const [error, setError] = useState('');
    const [refreshToken, setRefreshToken] = useState(0);
    const forceRefreshRef = useRef(false);

    function refresh() {
        forceRefreshRef.current = true;
        setRefreshToken((current) => current + 1);
    }

    useEffect(() => {
        setScreenshots([]);
        setStatus('idle');
        setError('');
    }, [worldId]);

    useEffect(() => {
        if (activeTab !== 'screenshots' || !worldId) {
            return undefined;
        }

        let active = true;
        let scanActive = false;
        let scanCompleted = false;
        let scanError = '';

        const loadWorldScreenshots = async () => {
            try {
                const result = await mediaRepository.getWorldScreenshots(
                    worldId
                );
                if (!active) {
                    return;
                }
                const screenshotList = Array.isArray(result)
                    ? (result as WorldWorldScreenshots)
                    : [];
                setScreenshots(screenshotList);
                if (scanError) {
                    setError(scanError);
                    setStatus(screenshotList.length ? 'ready' : 'error');
                    return;
                }
                setError('');
                setStatus('ready');
            } catch (loadError) {
                if (!active) {
                    return;
                }
                setScreenshots([]);
                setError(
                    loadError instanceof Error
                        ? loadError.message
                        : loadFailedMessage
                );
                setStatus('error');
            }
        };

        const completeScan = (scanStatus: ScreenshotScanStatus) => {
            if (scanCompleted) {
                return;
            }
            scanActive = false;
            scanCompleted = true;
            if (scanStatus?.error) {
                scanError = scanStatus.error;
            }
            void loadWorldScreenshots();
        };

        const handleScanStatus = (scanStatus: ScreenshotScanStatus) => {
            if (!active) {
                return;
            }
            if (scanStatus.error) {
                scanError = scanStatus.error;
            }
            if (scanStatus.running) {
                scanError = '';
                scanActive = true;
                scanCompleted = false;
                return;
            }
            if (scanActive) {
                completeScan(scanStatus);
            }
        };

        const unsubscribe =
            subscribeScreenshotLibraryScanStatus(handleScanStatus);
        setStatus('loading');
        setError('');
        const forceRefresh = forceRefreshRef.current;
        forceRefreshRef.current = false;
        const initializeScan = async () => {
            try {
                let currentStatus =
                    await getCurrentScreenshotLibraryScanStatus();
                if (!active) {
                    return;
                }
                if (!currentStatus) {
                    currentStatus =
                        await getCurrentScreenshotLibraryScanStatus();
                    if (!active) {
                        return;
                    }
                }
                if (currentStatus?.running) {
                    handleScanStatus(currentStatus);
                    return;
                }
                scanActive = true;
                const scanStatus =
                    await startScreenshotLibraryScan(forceRefresh);
                if (!active || !scanStatus) {
                    return;
                }
                handleScanStatus(scanStatus);
                if (!scanStatus.running) {
                    completeScan(scanStatus);
                }
            } catch (scanStartError) {
                if (!active) {
                    return;
                }
                setScreenshots([]);
                setError(
                    scanStartError instanceof Error
                        ? scanStartError.message
                        : loadFailedMessage
                );
                setStatus('error');
            }
        };
        void initializeScan();

        return () => {
            active = false;
            unsubscribe();
        };
    }, [activeTab, loadFailedMessage, openNonce, refreshToken, worldId]);

    return {
        error,
        refresh,
        refreshDisabled: status === 'loading',
        screenshots,
        status
    };
}

export { useWorldDialogScreenshots };
