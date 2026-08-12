import { useCallback, useEffect, useRef, useState } from 'react';
import type { Area, MediaSize, Point, Size } from 'react-easy-crop';

import type { CropResizeAxis } from './imageCropUtils';

const TRANSFORM_TRANSITION_MS = 180;
const ZOOM_DEFAULT = 1;

export type CropResizeHandle =
    | { kind: 'edge'; axis: CropResizeAxis; direction: 1 | -1 }
    | { kind: 'corner'; direction: Point };

interface ImageCropTransformState {
    crop: Point;
    cropSize: Size | null;
    fitWhole: boolean;
    flipH: boolean;
    flipV: boolean;
    rotation: number;
    zoom: number;
}

export function createImageCropTransformState(): ImageCropTransformState {
    return {
        crop: { x: 0, y: 0 },
        cropSize: null,
        fitWhole: false,
        flipH: false,
        flipV: false,
        rotation: 0,
        zoom: ZOOM_DEFAULT
    };
}

function prefersReducedMotion() {
    if (
        typeof window === 'undefined' ||
        typeof window.matchMedia !== 'function'
    ) {
        return false;
    }
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export function useImageCropDialogSession() {
    const originalImgRef = useRef<HTMLImageElement | null>(null);
    const previewScaleRef = useRef(1);
    const cropWrapperRef = useRef<HTMLDivElement | null>(null);
    const cropStageRef = useRef<HTMLDivElement | null>(null);
    const rotationInputRef = useRef<HTMLInputElement | null>(null);
    const [previewSrc, setPreviewSrc] = useState('');
    const [previewPending, setPreviewPending] = useState(false);
    const [cropperReady, setCropperReady] = useState(false);
    const [crop, setCrop] = useState<Point>({ x: 0, y: 0 });
    const [zoom, setZoom] = useState(ZOOM_DEFAULT);
    const [rotation, setRotation] = useState(0);
    const [rotationEditing, setRotationEditing] = useState(false);
    const [rotationInput, setRotationInput] = useState('');
    const [mediaSize, setMediaSize] = useState<MediaSize | null>(null);
    const [cropSize, setCropSize] = useState<Size | null>(null);
    const [flipH, setFlipH] = useState(false);
    const [flipV, setFlipV] = useState(false);
    const [fitWhole, setFitWhole] = useState(false);
    const [croppedAreaPixels, setCroppedAreaPixels] = useState<Area | null>(
        null
    );
    const [note, setNote] = useState('');
    const [cropWhiteBorder, setCropWhiteBorder] = useState(true);
    const [isConfirming, setIsConfirming] = useState(false);
    const [transformAnimating, setTransformAnimating] = useState(false);
    const transformAnimTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
        null
    );
    const rotationDragRef = useRef<{
        pointerId: number;
        centerX: number;
        centerY: number;
        previousAngleRadians: number;
        rotationDegrees: number;
    } | null>(null);
    const cropResizeDragRef = useRef<{
        pointerId: number;
        startX: number;
        startY: number;
        startSize: Size;
        handle: CropResizeHandle;
    } | null>(null);
    const trackpadPanningRef = useRef(false);
    const trackpadPanTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
        null
    );

    const resetTransforms = useCallback(() => {
        const state = createImageCropTransformState();
        setCrop(state.crop);
        setZoom(state.zoom);
        setRotation(state.rotation);
        setFlipH(state.flipH);
        setFlipV(state.flipV);
        setFitWhole(state.fitWhole);
        setRotationEditing(false);
        setCropSize(state.cropSize);
    }, []);

    useEffect(() => {
        const transformAnimTimer = transformAnimTimerRef;
        const trackpadPanTimer = trackpadPanTimerRef;
        return () => {
            if (transformAnimTimer.current) {
                clearTimeout(transformAnimTimer.current);
            }
            if (trackpadPanTimer.current) {
                clearTimeout(trackpadPanTimer.current);
            }
        };
    }, []);

    const triggerTransformAnim = useCallback(() => {
        if (prefersReducedMotion()) {
            return;
        }
        if (transformAnimTimerRef.current) {
            clearTimeout(transformAnimTimerRef.current);
        }
        setTransformAnimating(true);
        transformAnimTimerRef.current = setTimeout(() => {
            setTransformAnimating(false);
            transformAnimTimerRef.current = null;
        }, TRANSFORM_TRANSITION_MS);
    }, []);

    return {
        crop,
        croppedAreaPixels,
        cropResizeDragRef,
        cropSize,
        cropStageRef,
        cropWhiteBorder,
        cropWrapperRef,
        cropperReady,
        fitWhole,
        flipH,
        flipV,
        isConfirming,
        mediaSize,
        note,
        originalImgRef,
        previewPending,
        previewScaleRef,
        previewSrc,
        resetTransforms,
        rotation,
        rotationDragRef,
        rotationEditing,
        rotationInput,
        rotationInputRef,
        setCrop,
        setCroppedAreaPixels,
        setCropSize,
        setCropWhiteBorder,
        setCropperReady,
        setFitWhole,
        setFlipH,
        setFlipV,
        setIsConfirming,
        setMediaSize,
        setNote,
        setPreviewPending,
        setPreviewSrc,
        setRotation,
        setRotationEditing,
        setRotationInput,
        setTransformAnimating,
        setZoom,
        trackpadPanningRef,
        trackpadPanTimerRef,
        transformAnimating,
        transformAnimTimerRef,
        triggerTransformAnim,
        zoom
    };
}

export type ImageCropDialogSession = ReturnType<
    typeof useImageCropDialogSession
>;
