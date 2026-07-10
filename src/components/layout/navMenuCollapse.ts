import { useEffect, useState } from 'react';

export const NAV_MENU_COLLAPSE_DELAY_MS = 200;

export type AnimationFrameScheduler = Pick<
    Window,
    'cancelAnimationFrame' | 'requestAnimationFrame'
>;

export function scheduleKeyboardSidebarToggleCleanup(
    onFrame: () => void,
    scheduler: AnimationFrameScheduler = window
): () => void {
    const frameId = scheduler.requestAnimationFrame(onFrame);
    return () => {
        scheduler.cancelAnimationFrame(frameId);
    };
}

export function resolveDelayedNavMenuCollapsed(
    sidebarOpen: boolean,
    currentNavMenuCollapsed: boolean,
    elapsedMs: number,
    immediate = false
): boolean {
    if (immediate) {
        return !sidebarOpen;
    }
    if (elapsedMs < NAV_MENU_COLLAPSE_DELAY_MS) {
        return currentNavMenuCollapsed;
    }
    return !sidebarOpen;
}

export function useDelayedNavMenuCollapsed(
    sidebarOpen: boolean,
    immediate = false
): boolean {
    const [navMenuCollapsed, setNavMenuCollapsed] = useState(
        () => !sidebarOpen
    );

    useEffect(() => {
        if (immediate) {
            setNavMenuCollapsed(!sidebarOpen);
            return;
        }

        const timeoutId = window.setTimeout(() => {
            setNavMenuCollapsed((currentNavMenuCollapsed) =>
                resolveDelayedNavMenuCollapsed(
                    sidebarOpen,
                    currentNavMenuCollapsed,
                    NAV_MENU_COLLAPSE_DELAY_MS
                )
            );
        }, NAV_MENU_COLLAPSE_DELAY_MS);

        return () => {
            window.clearTimeout(timeoutId);
        };
    }, [immediate, sidebarOpen]);

    return immediate ? !sidebarOpen : navMenuCollapsed;
}
