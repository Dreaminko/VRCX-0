// @vitest-environment jsdom

import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
    createImageCropTransformState,
    useImageCropDialogSession
} from './imageCropDialogSession';

describe('imageCropDialogSession', () => {
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

    it('starts from the existing neutral transform', () => {
        expect(createImageCropTransformState()).toEqual({
            crop: { x: 0, y: 0 },
            cropSize: null,
            fitWhole: false,
            flipH: false,
            flipV: false,
            rotation: 0,
            zoom: 1
        });
    });

    it('resets the production session transform state', () => {
        const { result } = renderHook(() => useImageCropDialogSession());

        act(() => {
            result.current.setCrop({ x: 20, y: -10 });
            result.current.setCropSize({ width: 240, height: 160 });
            result.current.setFitWhole(true);
            result.current.setFlipH(true);
            result.current.setFlipV(true);
            result.current.setRotation(92);
            result.current.setRotationEditing(true);
            result.current.setZoom(2.5);
        });

        act(() => result.current.resetTransforms());

        expect({
            crop: result.current.crop,
            cropSize: result.current.cropSize,
            fitWhole: result.current.fitWhole,
            flipH: result.current.flipH,
            flipV: result.current.flipV,
            rotation: result.current.rotation,
            zoom: result.current.zoom
        }).toEqual(createImageCropTransformState());
        expect(result.current.rotationEditing).toBe(false);
    });

    it('clears transform and trackpad timers when the session unmounts', () => {
        const { result, unmount } = renderHook(() =>
            useImageCropDialogSession()
        );

        act(() => {
            result.current.triggerTransformAnim();
            result.current.trackpadPanTimerRef.current = setTimeout(
                () => undefined,
                1_000
            );
        });
        expect(vi.getTimerCount()).toBe(2);

        unmount();

        expect(vi.getTimerCount()).toBe(0);
    });
});
