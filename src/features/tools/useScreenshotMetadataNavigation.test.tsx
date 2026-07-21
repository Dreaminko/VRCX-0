// @vitest-environment jsdom

import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { useScreenshotMetadataNavigation } from './useScreenshotMetadataNavigation';

function createProps() {
    return {
        loadScreenshot: vi.fn<
            (path: string, withCarousel: boolean) => Promise<void>
        >(async () => {}),
        metadata: {
            nextFilePath: 'metadata-next.png',
            previousFilePath: 'metadata-prev.png'
        },
        onPathChange: undefined as ((path: string) => void) | undefined,
        searchNavigationPaths: [] as string[],
        selectedPath: '',
        setSelectedPath: vi.fn<(path: string) => void>()
    };
}

describe('useScreenshotMetadataNavigation', () => {
    it('wraps within search results and routes without loading twice', async () => {
        const props = createProps();
        props.searchNavigationPaths = ['a.png', 'b.png', 'c.png'];
        props.selectedPath = 'a.png';
        props.onPathChange = vi.fn();
        const { result, rerender } = renderHook(
            (currentProps) => useScreenshotMetadataNavigation(currentProps),
            { initialProps: props }
        );

        await act(() => result.current.navigatePrev());
        expect(props.setSelectedPath).toHaveBeenCalledWith('c.png');
        expect(props.onPathChange).toHaveBeenCalledWith('c.png');
        expect(props.loadScreenshot).not.toHaveBeenCalled();

        const lastProps = { ...props, selectedPath: 'c.png' };
        rerender(lastProps);
        await act(() => result.current.navigateNext());
        expect(props.setSelectedPath).toHaveBeenLastCalledWith('a.png');
        expect(props.onPathChange).toHaveBeenLastCalledWith('a.png');
    });

    it('falls back to metadata carousel paths outside search context', async () => {
        const props = createProps();
        const { result } = renderHook(() =>
            useScreenshotMetadataNavigation(props)
        );

        await act(() => result.current.navigatePrev());
        await act(() => result.current.navigateNext());

        expect(props.loadScreenshot).toHaveBeenNthCalledWith(
            1,
            'metadata-prev.png',
            true
        );
        expect(props.loadScreenshot).toHaveBeenNthCalledWith(
            2,
            'metadata-next.png',
            true
        );
    });

    it('falls back to metadata when the selected search path is stale', async () => {
        const props = createProps();
        props.searchNavigationPaths = ['a.png', 'b.png'];
        props.selectedPath = 'missing.png';
        const { result } = renderHook(() =>
            useScreenshotMetadataNavigation(props)
        );

        await act(() => result.current.navigateNext());

        expect(props.setSelectedPath).not.toHaveBeenCalled();
        expect(props.loadScreenshot).toHaveBeenCalledWith(
            'metadata-next.png',
            true
        );
    });

    it('uses the latest load callback without rebuilding navigation callbacks', async () => {
        const props = createProps();
        const firstLoad = props.loadScreenshot;
        const nextLoad = vi.fn(async () => {});
        const { result, rerender } = renderHook(
            (currentProps) => useScreenshotMetadataNavigation(currentProps),
            { initialProps: props }
        );
        const firstNavigateNext = result.current.navigateNext;

        rerender({ ...props, loadScreenshot: nextLoad });
        expect(result.current.navigateNext).toBe(firstNavigateNext);
        await act(() => result.current.navigateNext());

        expect(firstLoad).not.toHaveBeenCalled();
        expect(nextLoad).toHaveBeenCalledWith('metadata-next.png', true);
    });

    it('handles Alt+Arrow shortcuts and removes listeners on unmount', () => {
        const props = createProps();
        props.onPathChange = vi.fn();
        const { unmount } = renderHook(() =>
            useScreenshotMetadataNavigation(props)
        );
        const left = new KeyboardEvent('keydown', {
            altKey: true,
            cancelable: true,
            key: 'ArrowLeft'
        });
        const right = new KeyboardEvent('keydown', {
            altKey: true,
            cancelable: true,
            key: 'ArrowRight'
        });

        act(() => {
            window.dispatchEvent(left);
            window.dispatchEvent(right);
        });
        expect(left.defaultPrevented).toBe(true);
        expect(right.defaultPrevented).toBe(true);
        expect(props.onPathChange).toHaveBeenNthCalledWith(
            1,
            'metadata-prev.png'
        );
        expect(props.onPathChange).toHaveBeenNthCalledWith(
            2,
            'metadata-next.png'
        );

        unmount();
        window.dispatchEvent(
            new KeyboardEvent('keydown', {
                altKey: true,
                key: 'ArrowRight'
            })
        );
        expect(props.onPathChange).toHaveBeenCalledTimes(2);
    });
});
