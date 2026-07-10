import { describe, expect, it } from 'vitest';

import {
    NAV_MENU_COLLAPSE_DELAY_MS,
    resolveDelayedNavMenuCollapsed,
    scheduleKeyboardSidebarToggleCleanup
} from './navMenuCollapse';

describe('navMenuCollapse', () => {
    it('keeps expanded menu content during the sidebar collapse transition', () => {
        expect(resolveDelayedNavMenuCollapsed(false, false, 0)).toBe(false);
        expect(
            resolveDelayedNavMenuCollapsed(
                false,
                false,
                NAV_MENU_COLLAPSE_DELAY_MS - 1
            )
        ).toBe(false);
    });

    it('switches to collapsed menu content after the collapse transition', () => {
        expect(
            resolveDelayedNavMenuCollapsed(
                false,
                false,
                NAV_MENU_COLLAPSE_DELAY_MS
            )
        ).toBe(true);
    });

    it('keeps collapsed menu content during the sidebar expand transition', () => {
        expect(resolveDelayedNavMenuCollapsed(true, true, 0)).toBe(true);
        expect(
            resolveDelayedNavMenuCollapsed(
                true,
                true,
                NAV_MENU_COLLAPSE_DELAY_MS - 1
            )
        ).toBe(true);
    });

    it('switches to expanded menu content after the expand transition', () => {
        expect(
            resolveDelayedNavMenuCollapsed(
                true,
                true,
                NAV_MENU_COLLAPSE_DELAY_MS
            )
        ).toBe(false);
    });

    it('switches immediately for a keyboard collapse', () => {
        expect(resolveDelayedNavMenuCollapsed(false, false, 0, true)).toBe(
            true
        );
    });

    it('switches immediately for a keyboard expand', () => {
        expect(resolveDelayedNavMenuCollapsed(true, true, 0, true)).toBe(false);
    });

    it('cleans up the scheduled keyboard transition reset', () => {
        let frameCallback: FrameRequestCallback | null = null;
        let cancelledFrameId: number | null = null;
        const scheduler = {
            requestAnimationFrame(callback: FrameRequestCallback) {
                frameCallback = callback;
                return 7;
            },
            cancelAnimationFrame(frameId: number) {
                cancelledFrameId = frameId;
            }
        };
        const cleanup = scheduleKeyboardSidebarToggleCleanup(
            () => {},
            scheduler
        );

        expect(frameCallback).not.toBeNull();
        cleanup();
        expect(cancelledFrameId).toBe(7);
    });

    it('defers the keyboard transition reset until the next animation frame', () => {
        const frameCallbacks: FrameRequestCallback[] = [];
        let reset = false;
        const scheduler = {
            requestAnimationFrame(callback: FrameRequestCallback) {
                frameCallbacks.push(callback);
                return 8;
            },
            cancelAnimationFrame() {}
        };

        scheduleKeyboardSidebarToggleCleanup(() => {
            reset = true;
        }, scheduler);

        expect(reset).toBe(false);
        frameCallbacks[0]?.(0);
        expect(reset).toBe(true);
    });
});
