// @vitest-environment jsdom

import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { SidebarTabDisplayMode } from './side-panel/sidebarTabLayout';
import { useResponsiveSidePanelTabText } from './useResponsiveSidePanelTabText';

let availableWidth = 0;
let requiredTextWidth = 0;
let notifyResize: (() => void) | null = null;

class ResizeObserverMock {
    constructor(callback: ResizeObserverCallback) {
        notifyResize = () => callback([], this as unknown as ResizeObserver);
    }

    observe() {}
    unobserve() {}
    disconnect() {}
}

vi.stubGlobal('ResizeObserver', ResizeObserverMock);

function TestHarness({
    displayMode,
    tabLabels
}: {
    displayMode: SidebarTabDisplayMode;
    tabLabels: readonly string[];
}) {
    const { showTabText, tabListRef, tabViewportRef } =
        useResponsiveSidePanelTabText(displayMode, tabLabels);

    return (
        <div
            ref={tabViewportRef}
            data-testid="viewport"
            data-show-tab-text={showTabText}
        >
            <div ref={tabListRef} data-testid="tab-list" />
        </div>
    );
}

describe('useResponsiveSidePanelTabText', () => {
    beforeEach(() => {
        availableWidth = 0;
        requiredTextWidth = 0;
        notifyResize = null;

        Object.defineProperty(HTMLElement.prototype, 'clientWidth', {
            configurable: true,
            get() {
                return this.dataset.testid === 'viewport' ? availableWidth : 0;
            }
        });
        Object.defineProperty(HTMLElement.prototype, 'scrollWidth', {
            configurable: true,
            get() {
                return this.dataset.testid === 'tab-list'
                    ? requiredTextWidth
                    : 0;
            }
        });
    });

    afterEach(() => {
        cleanup();
        vi.restoreAllMocks();
    });

    it('collapses auto tabs when their rendered text does not fit', () => {
        availableWidth = 180;
        requiredTextWidth = 220;

        render(
            <TestHarness
                displayMode="auto"
                tabLabels={['Friends (65/471)', 'Groups (7)']}
            />
        );

        expect(screen.getByTestId('viewport').dataset.showTabText).toBe(
            'false'
        );

        availableWidth = 220;
        act(() => notifyResize?.());

        expect(screen.getByTestId('viewport').dataset.showTabText).toBe('true');
    });

    it('remeasures translated or dynamic labels while collapsed', () => {
        availableWidth = 180;
        requiredTextWidth = 220;
        const { rerender } = render(
            <TestHarness
                displayMode="auto"
                tabLabels={['Friends (65/471)', 'Groups (7)']}
            />
        );

        requiredTextWidth = 260;
        rerender(
            <TestHarness
                displayMode="auto"
                tabLabels={['Freunde (65/471)', 'Gruppen (7)']}
            />
        );

        availableWidth = 240;
        act(() => notifyResize?.());
        expect(screen.getByTestId('viewport').dataset.showTabText).toBe(
            'false'
        );

        availableWidth = 260;
        act(() => notifyResize?.());
        expect(screen.getByTestId('viewport').dataset.showTabText).toBe('true');
    });

    it('collapses icon and text mode when the rendered text does not fit', () => {
        availableWidth = 180;
        requiredTextWidth = 220;

        render(
            <TestHarness
                displayMode="iconText"
                tabLabels={['Friends', 'Groups']}
            />
        );

        expect(screen.getByTestId('viewport').dataset.showTabText).toBe(
            'false'
        );

        availableWidth = 220;
        act(() => notifyResize?.());
        expect(screen.getByTestId('viewport').dataset.showTabText).toBe('true');
    });

    it('keeps icon-only mode independent of available width', () => {
        availableWidth = 500;
        requiredTextWidth = 200;

        render(
            <TestHarness
                displayMode="iconOnly"
                tabLabels={['Friends', 'Groups']}
            />
        );

        expect(screen.getByTestId('viewport').dataset.showTabText).toBe(
            'false'
        );
    });

    it('keeps auto mode icon-only when more than two tabs are visible', () => {
        availableWidth = 500;
        requiredTextWidth = 200;

        render(
            <TestHarness
                displayMode="auto"
                tabLabels={['Friends', 'Groups', 'Favorites']}
            />
        );

        expect(screen.getByTestId('viewport').dataset.showTabText).toBe(
            'false'
        );
    });
});
