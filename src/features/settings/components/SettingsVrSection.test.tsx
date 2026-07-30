// @vitest-environment jsdom

import { render } from '@testing-library/react';
import type { ComponentProps } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createDefaultSettingsPrefs } from '../settingsDefaultPrefs';
import { SettingsVrSection } from './SettingsVrSection';

const captured = vi.hoisted(() => ({
    props: undefined as Record<string, unknown> | undefined
}));

vi.mock('./settings-tabs/SettingsVrTab', () => ({
    SettingsVrTab: (props: Record<string, unknown>) => {
        captured.props = props;
        return null;
    }
}));

type VrSectionProps = ComponentProps<typeof SettingsVrSection>['vr'];

function callback(name: string): (...args: unknown[]) => unknown {
    const value = captured.props?.[name];
    expect(value).toBeTypeOf('function');
    return value as (...args: unknown[]) => unknown;
}

function createVrSectionProps(): VrSectionProps {
    return {
        prefs: {
            ...createDefaultSettingsPrefs(),
            notificationTimeout: 3000
        },
        setHmdNotificationsDialogOpen: vi.fn(),
        setVrNotificationsDialogOpen: vi.fn(),
        setWristFeedNotificationsDialogOpen: vi.fn(),
        savePreferenceValue: vi.fn(),
        saveStringPreference: vi.fn(),
        saveBoolPreference: vi.fn(),
        setIntConfigPreference: vi.fn(),
        saveWristOverlayEnabled: vi.fn()
    };
}

describe('SettingsVrSection', () => {
    beforeEach(() => {
        captured.props = undefined;
    });

    it('persists notification number inputs with their config bounds', () => {
        const vr = createVrSectionProps();
        render(<SettingsVrSection vr={vr} />);

        callback('onNotificationTimeoutSecondsChange')('900');
        expect(vr.savePreferenceValue).toHaveBeenLastCalledWith(
            'notificationTimeout',
            600000,
            expect.any(Function)
        );
        const notificationTimeoutCommit = vi.mocked(vr.savePreferenceValue).mock
            .calls[0][2];
        notificationTimeoutCommit();
        expect(vr.setIntConfigPreference).toHaveBeenLastCalledWith(
            'notificationTimeout',
            600000,
            { min: 0, max: 600000, fallback: 3000 }
        );

        callback('onNotificationOpacityChange')(42.6);
        expect(vr.savePreferenceValue).toHaveBeenLastCalledWith(
            'notificationOpacity',
            43,
            expect.any(Function)
        );
        const notificationOpacityCommit = vi.mocked(vr.savePreferenceValue).mock
            .calls[1][2];
        notificationOpacityCommit();
        expect(vr.setIntConfigPreference).toHaveBeenLastCalledWith(
            'notificationOpacity',
            43,
            { min: 0, max: 100, fallback: 100 }
        );

        callback('onHmdNotificationTimeoutSecondsChange')('');
        expect(vr.savePreferenceValue).toHaveBeenLastCalledWith(
            'hmdNotificationTimeout',
            5000,
            expect.any(Function)
        );
        const hmdTimeoutCommit = vi.mocked(vr.savePreferenceValue).mock
            .calls[2][2];
        hmdTimeoutCommit();
        expect(vr.setIntConfigPreference).toHaveBeenLastCalledWith(
            'hmdNotificationTimeout',
            5000,
            { min: 1000, max: 30000, fallback: 5000 }
        );

        callback('onHmdNotificationOpacityChange')(-10);
        expect(vr.savePreferenceValue).toHaveBeenLastCalledWith(
            'hmdNotificationOpacity',
            0,
            expect.any(Function)
        );
    });

    it.each([
        ['onXsNotificationsChange', 'xsNotifications'],
        ['onOvrtHudNotificationsChange', 'ovrtHudNotifications'],
        ['onOvrtWristNotificationsChange', 'ovrtWristNotifications'],
        ['onImageNotificationsChange', 'imageNotifications'],
        ['onHmdNotificationsEnabledChange', 'hmdNotificationsEnabled'],
        ['onWristOverlayDarkBackgroundChange', 'wristOverlayDarkBackground'],
        [
            'onWristOverlayHidePrivateWorldsChange',
            'wristOverlayHidePrivateWorlds'
        ],
        ['onWristOverlayShowDevicesChange', 'wristOverlayShowDevices'],
        [
            'onWristOverlayShowBatteryPercentChange',
            'wristOverlayShowBatteryPercent'
        ]
    ])('maps %s to the %s boolean preference', (propName, key) => {
        const vr = createVrSectionProps();
        render(<SettingsVrSection vr={vr} />);

        callback(propName)(true);

        expect(vr.saveBoolPreference).toHaveBeenCalledWith(key, key, true);
    });

    it.each([
        ['onHmdNotificationStartModeChange', 'hmdNotificationStartMode'],
        ['onHmdNotificationPositionChange', 'hmdNotificationPosition'],
        ['onWristOverlayStartModeChange', 'wristOverlayStartMode'],
        ['onWristOverlayButtonChange', 'wristOverlayButton'],
        ['onWristOverlayHandChange', 'wristOverlayHand'],
        ['onWristOverlaySizeChange', 'wristOverlaySize']
    ])('maps %s to the %s string preference', (propName, key) => {
        const vr = createVrSectionProps();
        render(<SettingsVrSection vr={vr} />);

        callback(propName)('selected-value');

        expect(vr.saveStringPreference).toHaveBeenCalledWith(
            key,
            key,
            'selected-value'
        );
    });

    it('routes wrist enablement and filter-dialog actions to their owners', () => {
        const vr = createVrSectionProps();
        render(<SettingsVrSection vr={vr} />);

        callback('onWristOverlayEnabledChange')(true);
        callback('onOpenVrNotificationFiltersDialog')();
        callback('onOpenHmdNotificationFiltersDialog')();
        callback('onOpenWristFeedNotificationsDialog')();

        expect(vr.saveWristOverlayEnabled).toHaveBeenCalledWith(true);
        expect(vr.setVrNotificationsDialogOpen).toHaveBeenCalledWith(true);
        expect(vr.setHmdNotificationsDialogOpen).toHaveBeenCalledWith(true);
        expect(vr.setWristFeedNotificationsDialogOpen).toHaveBeenCalledWith(
            true
        );
    });
});
