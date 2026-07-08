import { useEffect, useState } from 'react';

export const NAV_MENU_COLLAPSE_DELAY_MS = 200;

export function resolveDelayedNavMenuCollapsed(
    sidebarOpen: boolean,
    currentNavMenuCollapsed: boolean,
    elapsedMs: number
): boolean {
    if (elapsedMs < NAV_MENU_COLLAPSE_DELAY_MS) {
        return currentNavMenuCollapsed;
    }
    return !sidebarOpen;
}

export function useDelayedNavMenuCollapsed(sidebarOpen: boolean): boolean {
    const [navMenuCollapsed, setNavMenuCollapsed] = useState(
        () => !sidebarOpen
    );

    useEffect(() => {
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
    }, [sidebarOpen]);

    return navMenuCollapsed;
}
