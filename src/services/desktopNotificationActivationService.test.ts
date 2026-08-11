import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    appTakePendingDesktopNotificationActivation:
        vi.fn<
            () => Promise<
                | import('@/platform/tauri/bindings').DesktopNotificationActivation
                | null
            >
        >(),
    eventHandlers: new Map<string, () => void>(),
    openUserDialog: vi.fn(),
    subscribe: vi.fn(),
    unsubscribe: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appTakePendingDesktopNotificationActivation:
            mocks.appTakePendingDesktopNotificationActivation
    }
}));

vi.mock('@/platform/tauri/client', () => ({
    tauriClient: {
        events: {
            subscribe: mocks.subscribe
        }
    }
}));

vi.mock('./dialogService', () => ({
    openUserDialog: mocks.openUserDialog
}));

import {
    bindDesktopNotificationActivationEvents,
    takePendingDesktopNotificationActivation
} from './desktopNotificationActivationService';

const USER_ID = 'usr_12345678-1234-1234-1234-1234567890ab';

describe('desktopNotificationActivationService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.eventHandlers.clear();
        mocks.appTakePendingDesktopNotificationActivation.mockResolvedValue(
            null
        );
        mocks.subscribe.mockImplementation(
            async (name: string, handler: () => void) => {
                mocks.eventHandlers.set(name, handler);
                return mocks.unsubscribe;
            }
        );
    });

    it('subscribes to activation wake events and takes the pending target', async () => {
        const unbind = await bindDesktopNotificationActivationEvents();
        mocks.appTakePendingDesktopNotificationActivation.mockResolvedValueOnce(
            { userId: USER_ID }
        );

        mocks.eventHandlers.get('desktopNotificationActivated')?.();

        await vi.waitFor(() => {
            expect(mocks.openUserDialog).toHaveBeenCalledWith({
                userId: USER_ID
            });
        });
        unbind();
        expect(mocks.unsubscribe).toHaveBeenCalledOnce();
    });

    it('opens a canonical pending user profile only once', async () => {
        mocks.appTakePendingDesktopNotificationActivation
            .mockResolvedValueOnce({ userId: USER_ID })
            .mockResolvedValueOnce(null);

        await takePendingDesktopNotificationActivation();
        await takePendingDesktopNotificationActivation();

        expect(mocks.openUserDialog).toHaveBeenCalledOnce();
        expect(mocks.openUserDialog).toHaveBeenCalledWith({ userId: USER_ID });
    });

    it('ignores an invalid user id returned across the IPC boundary', async () => {
        mocks.appTakePendingDesktopNotificationActivation.mockResolvedValueOnce(
            { userId: 'usr_invalid' }
        );

        await takePendingDesktopNotificationActivation();

        expect(mocks.openUserDialog).not.toHaveBeenCalled();
    });
});
