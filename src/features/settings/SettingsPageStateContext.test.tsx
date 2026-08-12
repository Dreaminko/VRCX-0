// @vitest-environment jsdom

import { act, render, screen } from '@testing-library/react';
import { useSyncExternalStore } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { SettingsPageStateSections } from './settingsPageStateSections';

const settingsState = vi.hoisted(() => {
    let current: unknown = null;
    const listeners = new Set<() => void>();
    return {
        getSnapshot: () => current,
        set(next: unknown) {
            current = next;
            for (const listener of listeners) {
                listener();
            }
        },
        subscribe(listener: () => void) {
            listeners.add(listener);
            return () => listeners.delete(listener);
        }
    };
});

vi.mock('./useSettingsPageState', () => ({
    useSettingsPageState: () =>
        useSyncExternalStore(
            settingsState.subscribe,
            settingsState.getSnapshot,
            settingsState.getSnapshot
        ) as SettingsPageStateSections
}));

import {
    SettingsPageStateProvider,
    useSettingsPageSection
} from './SettingsPageStateContext';

function createSections(
    purgeDialogOpen: boolean
): SettingsPageStateSections {
    const action = () => undefined;
    return {
        shell: {
            activeSettingsTab: 'system',
            setActiveSettingsTab: action,
            settingsTabs: []
        },
        system: {
            saveBoolPreference: action
        },
        interface: {},
        media: {},
        integrations: {},
        social: {},
        notifications: {},
        vr: {},
        advanced: {},
        dialogs: { purgeDialogOpen }
    } as unknown as SettingsPageStateSections;
}

describe('SettingsPageStateProvider', () => {
    beforeEach(() => {
        settingsState.set(createSections(false));
    });

    it('updates only consumers of the changed section', () => {
        let systemRenderCount = 0;
        let dialogsRenderCount = 0;

        function SystemConsumer() {
            useSettingsPageSection('system');
            systemRenderCount += 1;
            return <div>system</div>;
        }

        function DialogsConsumer() {
            const dialogs = useSettingsPageSection('dialogs');
            dialogsRenderCount += 1;
            return <div>{String(dialogs.purgeDialogOpen)}</div>;
        }

        render(
            <SettingsPageStateProvider>
                <SystemConsumer />
                <DialogsConsumer />
            </SettingsPageStateProvider>
        );

        expect(systemRenderCount).toBe(1);
        expect(dialogsRenderCount).toBe(1);
        expect(screen.getByText('system')).toBeTruthy();
        expect(screen.getByText('false')).toBeTruthy();

        act(() => {
            settingsState.set(createSections(false));
        });

        expect(systemRenderCount).toBe(1);
        expect(dialogsRenderCount).toBe(1);

        act(() => {
            settingsState.set(createSections(true));
        });

        expect(systemRenderCount).toBe(1);
        expect(dialogsRenderCount).toBe(2);
        expect(screen.getByText('true')).toBeTruthy();
    });
});
