import {
    type KeyboardEvent as ReactKeyboardEvent,
    type PointerEvent as ReactPointerEvent,
    useCallback
} from 'react';
import type { MediaSize, Point, Size } from 'react-easy-crop';

import type {
    CropResizeHandle,
    ImageCropDialogSession
} from './imageCropDialogSession';
import {
    CROP_ROTATION_GUTTER,
    constrainCropSizeToZoom,
    constrainCropToImage,
    getContinuousRotationDeltaDegrees,
    normalizeSignedRotation,
    resizeCropSize,
    resizeCropSizeFromCorner
} from './imageCropUtils';

const MIN_CROP_SHORT_EDGE = 56;
const TRACKPAD_PAN_END_MS = 160;
const TRACKPAD_PAN_THRESHOLD = 50;
const ZOOM_DEFAULT = 1;
const ZOOM_FACTOR = 1.2;

type ViewportSession = Pick<
    ImageCropDialogSession,
    | 'cropResizeDragRef'
    | 'cropStageRef'
    | 'resetTransforms'
    | 'rotationDragRef'
    | 'setCrop'
    | 'setCropSize'
    | 'setFitWhole'
    | 'setFlipH'
    | 'setFlipV'
    | 'setRotation'
    | 'setTransformAnimating'
    | 'setZoom'
    | 'trackpadPanningRef'
    | 'trackpadPanTimerRef'
    | 'transformAnimTimerRef'
    | 'triggerTransformAnim'
>;

export interface ImageCropViewportModel {
    aspect: number;
    constrainToImage: boolean;
    cropSize: Size | null;
    effectiveZoom: number;
    logZoomMax: number;
    logZoomMin: number;
    maxZoom: number;
    mediaSize: MediaSize | null;
    minZoom: number;
    rotation: number;
}

export function useImageCropViewportInteractions({
    model,
    session
}: {
    model: ImageCropViewportModel;
    session: ViewportSession;
}) {
    const {
        aspect,
        constrainToImage,
        cropSize,
        effectiveZoom,
        logZoomMax,
        logZoomMin,
        maxZoom,
        mediaSize,
        minZoom,
        rotation
    } = model;
    const {
        cropResizeDragRef,
        cropStageRef,
        resetTransforms,
        rotationDragRef,
        setCrop,
        setCropSize,
        setFitWhole,
        setFlipH,
        setFlipV,
        setRotation,
        setTransformAnimating,
        setZoom,
        trackpadPanningRef,
        trackpadPanTimerRef,
        transformAnimTimerRef,
        triggerTransformAnim
    } = session;

    const startCropRotation = useCallback(
        (event: ReactPointerEvent<HTMLSpanElement>) => {
            event.preventDefault();
            event.stopPropagation();
            const cropBounds =
                event.currentTarget.parentElement?.getBoundingClientRect();
            if (!cropBounds) {
                return;
            }
            if (transformAnimTimerRef.current) {
                clearTimeout(transformAnimTimerRef.current);
                transformAnimTimerRef.current = null;
            }
            setTransformAnimating(false);
            event.currentTarget.setPointerCapture(event.pointerId);
            const centerX = cropBounds.left + cropBounds.width / 2;
            const centerY = cropBounds.top + cropBounds.height / 2;
            rotationDragRef.current = {
                pointerId: event.pointerId,
                centerX,
                centerY,
                previousAngleRadians: Math.atan2(
                    event.clientY - centerY,
                    event.clientX - centerX
                ),
                rotationDegrees: rotation
            };
        },
        [
            rotation,
            rotationDragRef,
            setTransformAnimating,
            transformAnimTimerRef
        ]
    );

    const moveCropRotation = useCallback(
        (event: ReactPointerEvent<HTMLSpanElement>) => {
            const drag = rotationDragRef.current;
            if (!drag || drag.pointerId !== event.pointerId) {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
            const angleRadians = Math.atan2(
                event.clientY - drag.centerY,
                event.clientX - drag.centerX
            );
            drag.rotationDegrees += getContinuousRotationDeltaDegrees(
                drag.previousAngleRadians,
                angleRadians
            );
            drag.previousAngleRadians = angleRadians;
            setRotation(drag.rotationDegrees);
        },
        [rotationDragRef, setRotation]
    );

    const stopCropRotation = useCallback(
        (event: ReactPointerEvent<HTMLSpanElement>) => {
            const drag = rotationDragRef.current;
            if (!drag || drag.pointerId !== event.pointerId) {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
            rotationDragRef.current = null;
            if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                event.currentTarget.releasePointerCapture(event.pointerId);
            }
            setRotation((value) => normalizeSignedRotation(value));
        },
        [rotationDragRef, setRotation]
    );

    const adjustRotationFromKeyboard = useCallback(
        (event: ReactKeyboardEvent<HTMLSpanElement>) => {
            let rotationDelta = event.shiftKey ? 15 : 1;
            switch (event.key) {
                case 'ArrowLeft':
                case 'ArrowDown':
                    rotationDelta = -rotationDelta;
                    break;
                case 'ArrowRight':
                case 'ArrowUp':
                    break;
                case 'Home':
                    rotationDelta = 0;
                    break;
                default:
                    return;
            }
            event.preventDefault();
            event.stopPropagation();
            setRotation((value) =>
                rotationDelta === 0
                    ? 0
                    : normalizeSignedRotation(value + rotationDelta)
            );
        },
        [setRotation]
    );

    const startCropResize = useCallback(
        (
            handle: CropResizeHandle,
            event: ReactPointerEvent<HTMLSpanElement>
        ) => {
            if (!cropSize) {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
            event.currentTarget.setPointerCapture(event.pointerId);
            cropResizeDragRef.current = {
                pointerId: event.pointerId,
                startX: event.clientX,
                startY: event.clientY,
                startSize: cropSize,
                handle
            };
        },
        [cropResizeDragRef, cropSize]
    );

    const moveCropResize = useCallback(
        (event: ReactPointerEvent<HTMLSpanElement>) => {
            const drag = cropResizeDragRef.current;
            if (!drag || drag.pointerId !== event.pointerId) {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
            const cropStageBounds =
                cropStageRef.current?.getBoundingClientRect();
            if (!cropStageBounds) {
                return;
            }
            const cropBounds = {
                width: cropStageBounds.width,
                height: Math.max(
                    0,
                    cropStageBounds.height - CROP_ROTATION_GUTTER
                )
            };
            const delta = {
                x: event.clientX - drag.startX,
                y: event.clientY - drag.startY
            };
            const resized =
                drag.handle.kind === 'corner'
                    ? resizeCropSizeFromCorner(
                          drag.startSize,
                          delta,
                          drag.handle.direction,
                          cropBounds,
                          aspect,
                          MIN_CROP_SHORT_EDGE
                      )
                    : resizeCropSize(
                          drag.startSize,
                          drag.handle.axis,
                          (drag.handle.axis === 'horizontal'
                              ? delta.x
                              : delta.y) * drag.handle.direction,
                          cropBounds,
                          aspect,
                          MIN_CROP_SHORT_EDGE
                      );
            setCropSize(
                constrainToImage && mediaSize
                    ? constrainCropSizeToZoom(
                          resized,
                          mediaSize,
                          effectiveZoom,
                          rotation
                      )
                    : resized
            );
        },
        [
            aspect,
            constrainToImage,
            cropResizeDragRef,
            cropStageRef,
            effectiveZoom,
            mediaSize,
            rotation,
            setCropSize
        ]
    );

    const stopCropResize = useCallback(
        (event: ReactPointerEvent<HTMLSpanElement>) => {
            const drag = cropResizeDragRef.current;
            if (!drag || drag.pointerId !== event.pointerId) {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
            cropResizeDragRef.current = null;
            if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                event.currentTarget.releasePointerCapture(event.pointerId);
            }
        },
        [cropResizeDragRef]
    );

    const limitCropPosition = useCallback(
        (position: Point) =>
            constrainToImage && mediaSize && cropSize
                ? constrainCropToImage(
                      position,
                      mediaSize,
                      cropSize,
                      effectiveZoom,
                      rotation
                  )
                : position,
        [constrainToImage, cropSize, effectiveZoom, mediaSize, rotation]
    );

    const onCropChange = useCallback(
        (position: Point) => setCrop(limitCropPosition(position)),
        [limitCropPosition, setCrop]
    );

    const onWheelRequest = useCallback(
        (event: WheelEvent) => {
            if (event.ctrlKey) {
                trackpadPanningRef.current = false;
                if (trackpadPanTimerRef.current) {
                    clearTimeout(trackpadPanTimerRef.current);
                    trackpadPanTimerRef.current = null;
                }
                return true;
            }
            const startsTrackpadPan =
                event.deltaMode === 0 &&
                (event.deltaX !== 0 ||
                    Math.abs(event.deltaY) < TRACKPAD_PAN_THRESHOLD);
            if (!trackpadPanningRef.current && !startsTrackpadPan) {
                return true;
            }
            event.preventDefault();
            trackpadPanningRef.current = true;
            if (trackpadPanTimerRef.current) {
                clearTimeout(trackpadPanTimerRef.current);
            }
            trackpadPanTimerRef.current = setTimeout(() => {
                trackpadPanningRef.current = false;
                trackpadPanTimerRef.current = null;
            }, TRACKPAD_PAN_END_MS);
            setCrop((current) =>
                limitCropPosition({
                    x: current.x - event.deltaX,
                    y: current.y - event.deltaY
                })
            );
            return false;
        },
        [limitCropPosition, setCrop, trackpadPanningRef, trackpadPanTimerRef]
    );

    const rotateBy = useCallback(
        (delta: number) => setRotation((value) => value + delta),
        [setRotation]
    );
    const rotateLeft = useCallback(() => {
        triggerTransformAnim();
        rotateBy(-90);
    }, [rotateBy, triggerTransformAnim]);
    const rotateRight = useCallback(() => {
        triggerTransformAnim();
        rotateBy(90);
    }, [rotateBy, triggerTransformAnim]);
    const flipHorizontal = useCallback(() => {
        triggerTransformAnim();
        setFlipH((value) => !value);
    }, [setFlipH, triggerTransformAnim]);
    const flipVertical = useCallback(() => {
        triggerTransformAnim();
        setFlipV((value) => !value);
    }, [setFlipV, triggerTransformAnim]);
    const zoomIn = useCallback(() => {
        setZoom(Math.min(maxZoom, effectiveZoom * ZOOM_FACTOR));
    }, [effectiveZoom, maxZoom, setZoom]);
    const zoomOut = useCallback(() => {
        setZoom(Math.max(minZoom, effectiveZoom / ZOOM_FACTOR));
    }, [effectiveZoom, minZoom, setZoom]);
    const setZoomFromSlider = useCallback(
        (value: number | readonly number[]) => {
            const percentage = (Array.isArray(value) ? value[0] : value) ?? 0;
            setZoom(
                Math.exp(
                    logZoomMin + (percentage / 100) * (logZoomMax - logZoomMin)
                )
            );
        },
        [logZoomMax, logZoomMin, setZoom]
    );
    const toggleFit = useCallback(() => {
        setFitWhole((fitWhole) => {
            if (fitWhole) {
                setZoom((zoom) => Math.max(zoom, ZOOM_DEFAULT));
            }
            return !fitWhole;
        });
    }, [setFitWhole, setZoom]);

    return {
        adjustRotationFromKeyboard,
        flipHorizontal,
        flipVertical,
        moveCropResize,
        moveCropRotation,
        onCropChange,
        onWheelRequest,
        reset: resetTransforms,
        rotateLeft,
        rotateRight,
        setZoomFromSlider,
        startCropResize,
        startCropRotation,
        stopCropResize,
        stopCropRotation,
        toggleFit,
        zoomIn,
        zoomOut
    };
}
