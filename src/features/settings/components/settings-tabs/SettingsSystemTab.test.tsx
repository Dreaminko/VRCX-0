import type { ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) => key
    })
}));

type ChildrenProps = {
    children?: ReactNode;
};

vi.mock('@/ui/shadcn/badge', () => ({
    Badge: ({ children }: ChildrenProps) => <span>{children}</span>
}));

vi.mock('@/ui/shadcn/button', () => ({
    Button: ({ children }: ChildrenProps) => <button>{children}</button>
}));

vi.mock('@/ui/shadcn/switch', () => ({
    Switch: () => <span data-switch />
}));

vi.mock('../SettingsField', () => ({
    Field: ({
        children,
        description,
        label
    }: ChildrenProps & { description?: ReactNode; label?: ReactNode }) => (
        <section>
            <span>{label}</span>
            <span>{description}</span>
            {children}
        </section>
    ),
    SettingsGroup: ({ children }: ChildrenProps) => (
        <section>{children}</section>
    )
}));

vi.mock('../SettingsViewParts', () => ({
    SettingsTabContent: ({ children }: ChildrenProps) => <div>{children}</div>
}));

import { SettingsSystemTab } from './SettingsSystemTab';

function noop() {}

const handlers = {
    onAutoInstallUpdatesOnStartupChange: noop,
    onAutoLoginDelayEnabledChange: noop,
    onBackgroundModeDelayEnabledChange: noop,
    onBackgroundModeEnabledChange: noop,
    onCloseToTrayChange: noop,
    onPostUpdateChangelogToastChange: noop,
    onPromptAutoLoginDelaySeconds: noop,
    onPromptBackgroundModeDelayMinutes: noop,
    onProxyEnabledChange: noop,
    onProxySettings: noop,
    onStartAsMinimizedChange: noop,
    onStartAtWindowsStartupChange: noop,
    onSystemWindowFrameChange: noop
};

describe('SettingsSystemTab updater policy', () => {
    it('shows a disabled status badge instead of an update control', () => {
        const html = renderToStaticMarkup(
            <SettingsSystemTab
                updateCheckDisabled
                hostPlatform="windows"
                {...handlers}
            />
        );

        expect(html).toContain(
            'view.settings.general.application.check_for_updates_and_update'
        );
        expect(html).toContain(
            'view.settings.general.application.update_check_disabled'
        );
        expect(html).toContain(
            'view.settings.general.application.update_check_disabled_build_description'
        );
        expect(html).not.toContain(
            'view.settings.general.application.auto_install_updates_on_startup'
        );
    });
});
