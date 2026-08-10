import { useLayoutEffect, useRef, useState } from 'react';

import type { SidebarTabDisplayMode } from './side-panel/sidebarTabLayout';

type TabTextFit = {
    measurementKey: string;
    requiredTextWidth: number;
    textFits: boolean;
};

export function useResponsiveSidePanelTabText(
    displayMode: SidebarTabDisplayMode,
    tabLabels: readonly string[]
) {
    const measurementKey = tabLabels.join('\u0000');
    const tabViewportRef = useRef<HTMLDivElement | null>(null);
    const tabListRef = useRef<HTMLDivElement | null>(null);
    const [tabTextFit, setTabTextFit] = useState<TabTextFit>({
        measurementKey,
        requiredTextWidth: 0,
        textFits: true
    });
    const modeCanShowText =
        displayMode === 'iconText' ||
        (displayMode === 'auto' && tabLabels.length <= 2);
    const measurementIsCurrent = tabTextFit.measurementKey === measurementKey;
    const showTabText =
        modeCanShowText && (!measurementIsCurrent || tabTextFit.textFits);

    useLayoutEffect(() => {
        if (!modeCanShowText) {
            return;
        }

        const tabViewport = tabViewportRef.current;
        const tabList = tabListRef.current;
        if (!tabViewport || !tabList) {
            return;
        }

        function updateTabTextFit() {
            const currentTabViewport = tabViewportRef.current;
            const currentTabList = tabListRef.current;
            if (!currentTabViewport || !currentTabList) {
                return;
            }

            const availableWidth = currentTabViewport.clientWidth;
            const requiredTextWidth = showTabText
                ? currentTabList.scrollWidth
                : tabTextFit.requiredTextWidth;
            if (availableWidth <= 0 || requiredTextWidth <= 0) {
                return;
            }

            const textFits = requiredTextWidth <= availableWidth;
            setTabTextFit((current) => {
                if (
                    current.measurementKey === measurementKey &&
                    current.requiredTextWidth === requiredTextWidth &&
                    current.textFits === textFits
                ) {
                    return current;
                }
                return {
                    measurementKey,
                    requiredTextWidth,
                    textFits
                };
            });
        }

        updateTabTextFit();
        if (typeof ResizeObserver !== 'function') {
            return;
        }

        const resizeObserver = new ResizeObserver(updateTabTextFit);
        resizeObserver.observe(tabViewport);
        resizeObserver.observe(tabList);
        return () => resizeObserver.disconnect();
    }, [
        modeCanShowText,
        measurementKey,
        showTabText,
        tabTextFit.requiredTextWidth
    ]);

    return { showTabText, tabListRef, tabViewportRef };
}
