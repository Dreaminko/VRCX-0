import { useCallback, useEffect, useRef } from 'react';

import type { NormalizedScreenshotMetadata } from './screenshotMetadataValues';

export function useScreenshotMetadataNavigation({
    loadScreenshot,
    metadata,
    onPathChange,
    searchNavigationPaths,
    selectedPath,
    setSelectedPath
}: {
    loadScreenshot: (path: string, withCarousel: boolean) => Promise<void>;
    metadata: Pick<
        NormalizedScreenshotMetadata,
        | 'nextFilePath'
        | 'nextFolderPath'
        | 'previousFilePath'
        | 'previousFolderPath'
    > | null;
    onPathChange?: (path: string, folderPath?: string) => void;
    searchNavigationPaths: string[];
    selectedPath: string;
    setSelectedPath: (path: string) => void;
}) {
    const loadScreenshotRef = useRef(loadScreenshot);
    const searchNavigationIndex = searchNavigationPaths.indexOf(selectedPath);
    const isSearchNavigationActive = searchNavigationIndex !== -1;
    const canNavigatePrev =
        isSearchNavigationActive || Boolean(metadata?.previousFilePath);
    const canNavigateNext =
        isSearchNavigationActive || Boolean(metadata?.nextFilePath);

    useEffect(() => {
        loadScreenshotRef.current = loadScreenshot;
    }, [loadScreenshot]);

    const navigateToPath = useCallback(
        async (path: string) => {
            setSelectedPath(path);
            if (onPathChange) {
                onPathChange(path);
                return;
            }
            await loadScreenshotRef.current(path, false);
        },
        [onPathChange, setSelectedPath]
    );

    const navigateToMetadataPath = useCallback(
        async (path: string, folderPath: string) => {
            if (onPathChange) {
                if (folderPath) {
                    onPathChange(path, folderPath);
                } else {
                    onPathChange(path);
                }
                return;
            }
            await loadScreenshotRef.current(path, true);
        },
        [onPathChange]
    );

    const navigatePrev = useCallback(async () => {
        if (isSearchNavigationActive) {
            const prevIndex =
                searchNavigationIndex > 0
                    ? searchNavigationIndex - 1
                    : searchNavigationPaths.length - 1;
            const nextPath = searchNavigationPaths[prevIndex];
            await navigateToPath(nextPath);
            return;
        }

        if (metadata?.previousFilePath) {
            await navigateToMetadataPath(
                metadata.previousFilePath,
                metadata.previousFolderPath
            );
        }
    }, [
        isSearchNavigationActive,
        metadata?.previousFilePath,
        metadata?.previousFolderPath,
        navigateToMetadataPath,
        navigateToPath,
        searchNavigationIndex,
        searchNavigationPaths
    ]);

    const navigateNext = useCallback(async () => {
        if (isSearchNavigationActive) {
            const nextIndex =
                searchNavigationIndex < searchNavigationPaths.length - 1
                    ? searchNavigationIndex + 1
                    : 0;
            const nextPath = searchNavigationPaths[nextIndex];
            await navigateToPath(nextPath);
            return;
        }

        if (metadata?.nextFilePath) {
            await navigateToMetadataPath(
                metadata.nextFilePath,
                metadata.nextFolderPath
            );
        }
    }, [
        isSearchNavigationActive,
        metadata?.nextFilePath,
        metadata?.nextFolderPath,
        navigateToMetadataPath,
        navigateToPath,
        searchNavigationIndex,
        searchNavigationPaths
    ]);

    useEffect(() => {
        function handleKeyDown(event: KeyboardEvent) {
            if (!event.altKey) {
                return;
            }

            if (event.key === 'ArrowLeft') {
                event.preventDefault();
                navigatePrev();
            }

            if (event.key === 'ArrowRight') {
                event.preventDefault();
                navigateNext();
            }
        }

        window.addEventListener('keydown', handleKeyDown);
        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    }, [navigateNext, navigatePrev]);

    return {
        canNavigateNext,
        canNavigatePrev,
        navigateNext,
        navigatePrev
    };
}
