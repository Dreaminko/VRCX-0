import { describe, expect, it } from 'vitest';

import {
    NAV_MENU_COLLAPSE_DELAY_MS,
    resolveDelayedNavMenuCollapsed
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
});
