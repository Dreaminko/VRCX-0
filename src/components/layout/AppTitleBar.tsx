import { CopyIcon, MinusIcon, SquareIcon, XIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/utils';
import {
    closeWindow,
    minimizeWindow,
    toggleMaximizeWindow
} from '@/services/shellIntegrationService';

import { AppMenuBar } from './AppMenuBar';
import { TitleBarButton, useTitleBarActions } from './useTitleBarActions';
import { useWindowChromeState } from './useWindowChromeState';

async function runWindowAction(action: () => Promise<unknown>) {
    try {
        await action();
    } catch (error) {
        console.warn('Window control action failed:', error);
    }
}

function TitleBarWindowButton({ className, onAction, ...props }: any) {
    return (
        <TitleBarButton
            className={cn(
                'text-muted-foreground hover:text-foreground h-full w-9 rounded-none border-0',
                className
            )}
            onPointerDown={(event: React.PointerEvent) => {
                if (event.button === 0) {
                    onAction();
                }
            }}
            onClick={(event: React.MouseEvent) => {
                if (event.detail === 0) {
                    onAction();
                }
            }}
            {...props}
        />
    );
}

export function AppTitleBar() {
    const { t } = useTranslation();
    const {
        maximized: isMaximized,
        docked: isDocked,
        focused: isFocused
    } = useWindowChromeState();
    const {
        isSessionReady,
        actions,
        quickSearchDialog,
        openQuickSearch,
        openDirectAccessFromClipboard,
        openNotificationCenter,
        toggleRightSidebar,
        rightSidebarOpen
    } = useTitleBarActions('px-1');

    const MaximizeIcon = isMaximized ? CopyIcon : SquareIcon;
    const maximizeLabel = isMaximized ? 'Restore window' : 'Maximize window';

    return (
        <>
            <header
                data-app-titlebar="true"
                data-window-docked={isDocked || undefined}
                data-window-blurred={!isFocused || undefined}
                data-vrcx-0-surface="titlebar"
                className="vrcx-0-titlebar text-foreground pointer-events-auto relative z-[60] flex h-8 shrink-0 items-center border-b select-none"
            >
                <div
                    data-tauri-drag-region
                    className="flex h-full min-w-0 flex-1 items-center gap-2 px-3"
                >
                    {isSessionReady ? (
                        <div
                            data-titlebar-interactive="true"
                            className="h-full shrink-0"
                            onMouseDown={(event) => {
                                event.stopPropagation();
                            }}
                            onDoubleClick={(event) => {
                                event.stopPropagation();
                            }}
                        >
                            <AppMenuBar
                                rightSidebarOpen={rightSidebarOpen}
                                onOpenQuickSearch={openQuickSearch}
                                onOpenDirectAccess={
                                    openDirectAccessFromClipboard
                                }
                                onOpenNotificationCenter={
                                    openNotificationCenter
                                }
                                onToggleRightSidebar={toggleRightSidebar}
                            />
                        </div>
                    ) : null}
                    <div
                        data-tauri-drag-region
                        className="h-full min-w-0 flex-1"
                    />
                </div>
                {actions}
                <div className="flex h-full shrink-0 items-center">
                    <TitleBarWindowButton
                        label={t('app_menu.label.minimize_window')}
                        onAction={() => {
                            runWindowAction(minimizeWindow);
                        }}
                    >
                        <MinusIcon data-icon="inline-start" />
                    </TitleBarWindowButton>
                    <TitleBarWindowButton
                        label={maximizeLabel}
                        onAction={() => {
                            runWindowAction(toggleMaximizeWindow);
                        }}
                    >
                        <MaximizeIcon data-icon="inline-start" />
                    </TitleBarWindowButton>
                    <TitleBarWindowButton
                        label={t('app_menu.action.close_window')}
                        className="hover:bg-destructive! hover:text-destructive-foreground!"
                        onAction={() => {
                            runWindowAction(closeWindow);
                        }}
                    >
                        <XIcon data-icon="inline-start" />
                    </TitleBarWindowButton>
                </div>
            </header>
            {quickSearchDialog}
        </>
    );
}
