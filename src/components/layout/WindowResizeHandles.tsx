import type { WindowResizeDirection } from '@/platform/tauri/webview';
import { startResizeDraggingWindow } from '@/services/shellIntegrationService';

import { useWindowChromeState } from './useWindowChromeState';

interface ResizeHandle {
    direction: WindowResizeDirection;
    className: string;
}

const CORNER_HANDLES: ResizeHandle[] = [
    {
        direction: 'NorthWest',
        className: 'top-0 left-0 size-4 cursor-nwse-resize'
    },
    {
        direction: 'NorthEast',
        className: 'top-0 right-0 size-4 cursor-nesw-resize'
    },
    {
        direction: 'SouthWest',
        className: 'bottom-0 left-0 size-4 cursor-nesw-resize'
    },
    {
        direction: 'SouthEast',
        className: 'right-0 bottom-0 size-4 cursor-nwse-resize'
    }
];

const EDGE_HANDLES: ResizeHandle[] = [
    {
        direction: 'North',
        className: 'top-0 right-4 left-4 h-2.5 cursor-ns-resize'
    },
    {
        direction: 'South',
        className: 'right-4 bottom-0 left-4 h-2.5 cursor-ns-resize'
    },
    {
        direction: 'West',
        className: 'top-4 bottom-4 left-0 w-2.5 cursor-ew-resize'
    },
    {
        direction: 'East',
        className: 'top-4 right-0 bottom-4 w-2.5 cursor-ew-resize'
    }
];

export function WindowResizeHandles() {
    const { docked } = useWindowChromeState();

    if (docked) {
        return null;
    }

    return (
        <div className="pointer-events-none fixed inset-0 z-[100]">
            {[...EDGE_HANDLES, ...CORNER_HANDLES].map((handle) => (
                <div
                    key={handle.direction}
                    className={`pointer-events-auto absolute ${handle.className}`}
                    onPointerDown={(event) => {
                        if (event.button !== 0) {
                            return;
                        }
                        event.preventDefault();
                        startResizeDraggingWindow(handle.direction).catch(
                            () => undefined
                        );
                    }}
                />
            ))}
        </div>
    );
}
