// @vitest-environment jsdom

import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mediaMocks = vi.hoisted(() => ({
    equipProfileDecoration: vi.fn(),
    collectInventoryItems: vi.fn(),
    unequipProfileDecoration: vi.fn()
}));

const maintenanceMocks = vi.hoisted(() => ({
    refreshCurrentUser: vi.fn()
}));

const runtimeState = vi.hoisted(() => ({
    auth: {
        currentUserEndpoint: 'https://api.vrchat.cloud/api/1',
        currentUserId: 'usr_self',
        currentUserWebsocket: 'wss://pipeline.vrchat.cloud'
    }
}));

const toastMocks = vi.hoisted(() => ({
    error: vi.fn(),
    success: vi.fn()
}));

vi.mock('@/repositories/mediaRepository', () => ({
    default: mediaMocks
}));
vi.mock('@/services/backgroundMaintenanceSessionService', () => ({
    refreshCurrentUser: maintenanceMocks.refreshCurrentUser
}));
vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: (selector: (state: typeof runtimeState) => unknown) =>
        selector(runtimeState)
}));
vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) => key
    })
}));
vi.mock('sonner', () => ({
    toast: toastMocks
}));

import { useUserDialogProfileDecorations } from './useUserDialogProfileDecorations';

function equippableItem(overrides: Record<string, unknown> = {}) {
    return {
        id: 'inv_frame',
        holderId: 'usr_self',
        itemType: 'iconFrame',
        equipSlot: '',
        equipSlots: ['iconFrame'],
        flags: ['equippable'],
        ...overrides
    };
}

describe('useUserDialogProfileDecorations', () => {
    beforeEach(() => {
        vi.resetAllMocks();
        mediaMocks.collectInventoryItems.mockResolvedValue({
            items: [],
            truncated: false
        });
        mediaMocks.equipProfileDecoration.mockResolvedValue({
            json: { ok: true }
        });
        mediaMocks.unequipProfileDecoration.mockResolvedValue({ json: 'OK' });
        maintenanceMocks.refreshCurrentUser.mockResolvedValue({
            id: 'usr_self'
        });
    });
    afterEach(cleanup);

    it('loads owned decorations and groups them by slot', async () => {
        mediaMocks.collectInventoryItems
            .mockResolvedValueOnce({
                items: [
                    { id: 'inv_frame', itemType: 'iconFrame' },
                    { id: 'inv_effect', itemType: 'profileEffect' },
                    { id: 'inv_plate', itemType: 'nameplateEffect' },
                    { id: 'inv_other', itemType: 'droneskin' }
                ],
                truncated: false
            })
            .mockResolvedValue({ items: [], truncated: false });

        const { result } = renderHook(() =>
            useUserDialogProfileDecorations({ enabled: true })
        );

        await waitFor(() => expect(result.current.isReady).toBe(true));

        expect(mediaMocks.collectInventoryItems).toHaveBeenCalledWith({
            order: 'newest',
            types: 'iconFrame,profileEffect,nameplateEffect',
            notFlags: 'ugc',
            archived: false
        });
        expect(result.current.itemsBySlot.iconFrame).toHaveLength(1);
        expect(result.current.itemsBySlot.profileEffect).toHaveLength(1);
        expect(result.current.itemsBySlot.nameplateEffect).toHaveLength(1);
    });

    it('does not load while disabled', () => {
        renderHook(() => useUserDialogProfileDecorations({ enabled: false }));
        expect(mediaMocks.collectInventoryItems).not.toHaveBeenCalled();
    });

    it('equips an unequipped item and refreshes the self profile', async () => {
        const { result } = renderHook(() =>
            useUserDialogProfileDecorations({ enabled: true })
        );
        await waitFor(() => expect(result.current.isReady).toBe(true));

        await act(async () => {
            result.current.equipItem(equippableItem());
        });

        await waitFor(() =>
            expect(mediaMocks.equipProfileDecoration).toHaveBeenCalledWith({
                expectedUserId: 'usr_self',
                inventoryId: 'inv_frame',
                equipSlot: 'iconFrame'
            })
        );
        expect(maintenanceMocks.refreshCurrentUser).toHaveBeenCalledWith({
            expectedUserId: 'usr_self',
            expectedEndpoint: 'https://api.vrchat.cloud/api/1',
            expectedWebsocket: 'wss://pipeline.vrchat.cloud'
        });
        expect(toastMocks.success).toHaveBeenCalledWith(
            'dialog.inventory.equipped_success'
        );
    });

    it('unequips a slot directly', async () => {
        const { result } = renderHook(() =>
            useUserDialogProfileDecorations({ enabled: true })
        );
        await waitFor(() => expect(result.current.isReady).toBe(true));

        await act(async () => {
            result.current.unequipSlot('profileEffect');
        });

        await waitFor(() =>
            expect(mediaMocks.unequipProfileDecoration).toHaveBeenCalledWith({
                expectedUserId: 'usr_self',
                equipSlot: 'profileEffect'
            })
        );
        expect(mediaMocks.equipProfileDecoration).not.toHaveBeenCalled();
    });

    it('ignores equipItem for an already-equipped item', async () => {
        const { result } = renderHook(() =>
            useUserDialogProfileDecorations({ enabled: true })
        );
        await waitFor(() => expect(result.current.isReady).toBe(true));

        await act(async () => {
            result.current.equipItem(
                equippableItem({
                    id: 'inv_equipped',
                    itemType: 'profileEffect',
                    equipSlot: 'profileEffect',
                    equipSlots: ['profileEffect']
                })
            );
        });

        expect(mediaMocks.equipProfileDecoration).not.toHaveBeenCalled();
        expect(mediaMocks.unequipProfileDecoration).not.toHaveBeenCalled();
    });
});
