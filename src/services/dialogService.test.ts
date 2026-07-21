import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('sonner', () => ({
    toast: {
        info: vi.fn()
    }
}));

vi.mock('@/services/i18nService', () => ({
    default: {
        t: (key: string) => key
    }
}));

vi.mock('@/services/userFactAccessService', () => ({
    recordUserProfile: vi.fn()
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: {
        getState: () => ({
            auth: {
                currentUserEndpoint: 'https://api.vrchat.cloud/api/1'
            }
        })
    }
}));

import { useDialogStore } from '@/state/dialogStore';

import {
    openAvatarDialog,
    openUserDialog,
    openWorldDialog
} from './dialogService';

describe('dialogService entity trail', () => {
    beforeEach(() => {
        useDialogStore.getState().clearDialogState();
    });

    it('truncates a dialog cycle when reopening an entity already in the trail', () => {
        openUserDialog({ userId: 'usr_a', title: 'User A' });
        openWorldDialog({ worldId: 'wrld_w', title: 'World W' });

        openUserDialog({ userId: 'usr_a', title: 'User A' });

        const state = useDialogStore.getState();
        expect(state.activeDialog).toMatchObject({
            kind: 'user',
            entityId: 'usr_a'
        });
        expect(state.breadcrumbs.map((crumb) => crumb.key)).toEqual([
            'user:usr_a'
        ]);
    });

    it('replaces the retained crumb with metadata from the latest open request', () => {
        openUserDialog({ userId: 'usr_a', title: 'User A' });
        openWorldDialog({ worldId: 'wrld_w', title: 'Old World' });
        openAvatarDialog({ avatarId: 'avtr_v', title: 'Avatar V' });

        openWorldDialog({
            worldId: 'wrld_w',
            title: 'Fresh World',
            seedData: { id: 'wrld_w', name: 'Fresh World' }
        });

        const state = useDialogStore.getState();
        expect(state.breadcrumbs.map((crumb) => crumb.key)).toEqual([
            'user:usr_a',
            'world:wrld_w'
        ]);
        expect(state.breadcrumbs.at(-1)).toMatchObject({
            title: 'Fresh World',
            payload: {
                seedData: { id: 'wrld_w', name: 'Fresh World' }
            }
        });
    });
});
