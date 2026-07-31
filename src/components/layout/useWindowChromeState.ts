import { useEffect, useRef, useState } from 'react';

import { tauriEvents } from '@/platform/tauri/events';
import { isWindowMaximized } from '@/services/shellIntegrationService';

const WINDOW_CHROME_STATE_EVENT = 'windowChromeState';

export interface WindowChromeState {
    maximized: boolean;
    docked: boolean;
    focused: boolean;
}

export function useWindowChromeState(): WindowChromeState {
    const [state, setState] = useState<WindowChromeState>({
        maximized: false,
        docked: false,
        focused: true
    });
    const hasHostStateRef = useRef(false);

    useEffect(() => {
        let disposed = false;
        let unsubscribe: (() => void) | null = null;

        isWindowMaximized()
            .then((maximized) => {
                if (disposed || hasHostStateRef.current) {
                    return;
                }
                setState((previous) => ({
                    ...previous,
                    maximized,
                    docked: maximized
                }));
            })
            .catch(() => undefined);

        tauriEvents
            .subscribe<WindowChromeState>(
                WINDOW_CHROME_STATE_EVENT,
                (payload) => {
                    if (disposed) {
                        return;
                    }
                    hasHostStateRef.current = true;
                    setState(payload);
                }
            )
            .then((dispose) => {
                if (disposed) {
                    dispose();
                    return;
                }
                unsubscribe = dispose;
            })
            .catch(() => undefined);

        return () => {
            disposed = true;
            unsubscribe?.();
        };
    }, []);

    return state;
}
