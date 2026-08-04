import { useCallback, useEffect, useState } from 'react';

import { setRightSidebarOpenPreference } from '@/services/preferencesService';
import { useShellStore } from '@/state/shellStore';

import { getDefaultHiddenSidePanelPath } from './sidePanelRoutes';

const sidePanelRouteOpenStateStorageKey =
    'vrcx-main-layout-right-sidebar-route-open-state';
const sidePanelRouteOpenStateEvent =
    'vrcx-main-layout-right-sidebar-route-open-state-change';

type SidePanelRouteOpenState = Record<string, boolean>;

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function readSidePanelRouteOpenState(): SidePanelRouteOpenState {
    if (typeof window === 'undefined') {
        return {};
    }
    try {
        const value = JSON.parse(
            window.localStorage.getItem(sidePanelRouteOpenStateStorageKey) ||
                '{}'
        );
        if (!value || typeof value !== 'object' || Array.isArray(value)) {
            return {};
        }
        return Object.fromEntries(
            Object.entries(value).filter(
                (entry): entry is [string, boolean] =>
                    typeof entry[1] === 'boolean'
            )
        );
    } catch {
        return {};
    }
}

function writeSidePanelRouteOpenState(routeKey: string, open: boolean) {
    if (typeof window === 'undefined') {
        return;
    }

    const nextState: SidePanelRouteOpenState = {
        ...readSidePanelRouteOpenState(),
        [routeKey]: Boolean(open)
    };

    try {
        window.localStorage.setItem(
            sidePanelRouteOpenStateStorageKey,
            JSON.stringify(nextState)
        );
    } catch {
        // no-op
    }

    window.dispatchEvent(
        new CustomEvent(sidePanelRouteOpenStateEvent, {
            detail: { routeKey, open: Boolean(open) }
        })
    );
}

export function useRightSidePanelVisibility(pathname: string) {
    const routeKey = getDefaultHiddenSidePanelPath(pathname);
    const rightSidebarOpen = useShellStore((state) => state.rightSidebarOpen);
    const [routeOpenState, setRouteOpenState] = useState(
        readSidePanelRouteOpenState
    );
    const sidePanelOpen = routeKey
        ? routeOpenState[routeKey] === true
        : rightSidebarOpen;

    useEffect(() => {
        if (typeof window === 'undefined') {
            return undefined;
        }

        const handleRouteStateChange = (event: Event) => {
            const detail =
                event instanceof CustomEvent && isRecord(event.detail)
                    ? event.detail
                    : null;
            if (detail && typeof detail.routeKey === 'string') {
                const routeKey = detail.routeKey;
                setRouteOpenState((currentState) => ({
                    ...currentState,
                    [routeKey]: detail.open === true
                }));
                return;
            }
            setRouteOpenState(readSidePanelRouteOpenState());
        };
        const handleStorage = (event: StorageEvent) => {
            if (
                event.key === sidePanelRouteOpenStateStorageKey ||
                event.key === null
            ) {
                setRouteOpenState(readSidePanelRouteOpenState());
            }
        };

        window.addEventListener(
            sidePanelRouteOpenStateEvent,
            handleRouteStateChange
        );
        window.addEventListener('storage', handleStorage);
        return () => {
            window.removeEventListener(
                sidePanelRouteOpenStateEvent,
                handleRouteStateChange
            );
            window.removeEventListener('storage', handleStorage);
        };
    }, []);

    const setSidePanelOpen = useCallback(
        (open: boolean) => {
            if (routeKey) {
                writeSidePanelRouteOpenState(routeKey, open);
                return;
            }
            void setRightSidebarOpenPreference(open);
        },
        [routeKey]
    );

    const toggleSidePanelOpen = useCallback(() => {
        setSidePanelOpen(!sidePanelOpen);
    }, [setSidePanelOpen, sidePanelOpen]);

    return {
        routeKey,
        sidePanelOpen,
        setSidePanelOpen,
        toggleSidePanelOpen
    };
}
