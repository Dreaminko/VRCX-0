// @vitest-environment jsdom

import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useImageCropDialogSession } from './imageCropDialogSession';
import { useImageCropViewportInteractions } from './useImageCropViewportInteractions';

function useViewportHarness() {
    const session = useImageCropDialogSession();
    const interactions = useImageCropViewportInteractions({
        model: {
            aspect: 1,
            constrainToImage: false,
            cropSize: session.cropSize,
            effectiveZoom: session.zoom,
            logZoomMax: Math.log(5),
            logZoomMin: Math.log(0.3),
            maxZoom: 5,
            mediaSize: null,
            minZoom: 0.3,
            rotation: session.rotation
        },
        session
    });
    return { interactions, session };
}

describe('useImageCropViewportInteractions', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.stubGlobal(
            'matchMedia',
            vi.fn(() => ({
                matches: false,
                addEventListener: vi.fn(),
                removeEventListener: vi.fn()
            }))
        );
    });

    afterEach(() => {
        vi.unstubAllGlobals();
        vi.useRealTimers();
    });

    it('applies viewport rotation and zoom through the production session', () => {
        const { result } = renderHook(() => useViewportHarness());

        act(() => {
            result.current.interactions.rotateRight();
            result.current.interactions.zoomIn();
        });

        expect(result.current.session.rotation).toBe(90);
        expect(result.current.session.zoom).toBe(1.2);
        expect(result.current.session.transformAnimating).toBe(true);

        act(() => vi.advanceTimersByTime(180));

        expect(result.current.session.transformAnimating).toBe(false);
    });
});
