// @vitest-environment jsdom

import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { UserDialogProfileRecord } from './useUserDialogProfileResource';

const mocks = vi.hoisted(() => ({
    addTags: vi.fn(),
    removeTags: vi.fn(),
    setAuthBootstrap: vi.fn(),
    toastError: vi.fn(),
    toastSuccess: vi.fn(),
    updateBadge: vi.fn(),
    updateCurrentUser: vi.fn()
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock('sonner', () => ({
    toast: {
        error: mocks.toastError,
        success: mocks.toastSuccess
    }
}));

vi.mock('@/services/currentUserProfileService', () => ({
    default: {
        addCurrentUserTags: mocks.addTags,
        removeCurrentUserTags: mocks.removeTags,
        updateCurrentUser: mocks.updateCurrentUser
    }
}));

vi.mock('@/repositories/userProfileRepository', () => ({
    default: { updateCurrentUserBadge: mocks.updateBadge }
}));

vi.mock('@/state/runtimeStore', () => {
    const useRuntimeStore = Object.assign(vi.fn(), {
        getState: () => ({
            auth: {
                currentUserSnapshot: {
                    id: 'usr_self',
                    displayName: 'Stored User',
                    status: 'active'
                }
            },
            setAuthBootstrap: mocks.setAuthBootstrap
        })
    });
    return { useRuntimeStore };
});

vi.mock('@/state/vrchatConfigStore', () => ({
    useVrchatConfigStore: (selector: (state: { snapshot: null }) => unknown) =>
        selector({ snapshot: null })
}));

vi.mock('./useCurrentUserSocialStatusDialog', () => ({
    useCurrentUserSocialStatusDialog: () => ({
        dialog: { open: false },
        openDialog: vi.fn()
    })
}));

import { useUserDialogSelfActions } from './useUserDialogSelfActions';

const profile: UserDialogProfileRecord = {
    id: 'usr_self',
    displayName: 'Current User',
    allowAvatarCopying: false,
    bio: 'Old bio',
    bioLinks: ['https://old.example'],
    pronouns: 'they/them',
    tags: ['language_en']
};

function renderActions() {
    const actionStatusRef = { current: 'idle' };
    const setActionStatus = vi.fn();
    const setBaseProfile = vi.fn();
    const hook = renderHook(() =>
        useUserDialogSelfActions({
            profile,
            isCurrentUser: true,
            currentUserId: 'usr_self',
            currentUserSnapshot: profile,
            currentEndpoint: 'https://api.vrchat.cloud/api/1',
            baseProfile: profile,
            setBaseProfile,
            actionStatusRef,
            setActionStatus
        })
    );
    return {
        ...hook,
        actionStatusRef,
        setActionStatus,
        setBaseProfile
    };
}

describe('useUserDialogSelfActions', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('locks concurrent profile mutations and commits the resolved snapshot', async () => {
        let resolveUpdate!: (value: UserDialogProfileRecord) => void;
        mocks.updateCurrentUser.mockReturnValue(
            new Promise((resolve) => {
                resolveUpdate = resolve;
            })
        );
        const rendered = renderActions();

        const first = rendered.result.current.actions.toggleSelfAvatarCopying();
        const second =
            rendered.result.current.actions.toggleSelfAvatarCopying();

        expect(rendered.actionStatusRef.current).toBe('self-profile');
        expect(mocks.updateCurrentUser).toHaveBeenCalledOnce();
        await expect(second).resolves.toBeUndefined();

        const nextProfile = {
            ...profile,
            allowAvatarCopying: true
        };
        await act(async () => resolveUpdate(nextProfile));
        await first;

        expect(rendered.setBaseProfile).toHaveBeenCalled();
        expect(mocks.setAuthBootstrap).toHaveBeenCalledWith(
            expect.objectContaining({
                currentUserId: 'usr_self',
                currentUserSnapshot: expect.objectContaining({
                    allowAvatarCopying: true
                })
            })
        );
        expect(rendered.actionStatusRef.current).toBe('idle');
        expect(mocks.toastSuccess).toHaveBeenCalledWith(
            'dialog.user.success.avatar_cloning_setting_updated'
        );
    });

    it('unlocks and preserves the displayed profile when a mutation fails', async () => {
        mocks.updateCurrentUser.mockRejectedValue(new Error('update failed'));
        const rendered = renderActions();

        await act(async () =>
            rendered.result.current.actions.toggleSelfBooping()
        );

        expect(rendered.setBaseProfile).not.toHaveBeenCalled();
        expect(mocks.setAuthBootstrap).not.toHaveBeenCalled();
        expect(rendered.actionStatusRef.current).toBe('idle');
        expect(mocks.toastError).toHaveBeenCalledWith('update failed');
    });

    it('applies profile fields and language removals before additions', async () => {
        mocks.updateCurrentUser.mockResolvedValue({
            ...profile,
            bio: 'New bio'
        });
        mocks.removeTags.mockResolvedValue({
            ...profile,
            bio: 'New bio',
            tags: []
        });
        mocks.addTags.mockResolvedValue({
            ...profile,
            bio: 'New bio',
            tags: ['language_ja']
        });
        const rendered = renderActions();

        act(() => rendered.result.current.actions.editSelfProfileDetails());
        act(() =>
            rendered.result.current.profileDetailsDialog.setDraft({
                languageKeys: ['ja'],
                bio: 'New bio',
                bioLinks: ['https://new.example'],
                pronouns: 'she/her'
            })
        );
        await act(async () =>
            rendered.result.current.profileDetailsDialog.onSave()
        );

        expect(mocks.updateCurrentUser).toHaveBeenCalledWith({
            userId: 'usr_self',
            params: {
                bio: 'New bio',
                bioLinks: ['https://new.example'],
                pronouns: 'she/her'
            }
        });
        expect(mocks.removeTags).toHaveBeenCalledWith({
            userId: 'usr_self',
            tags: ['language_en']
        });
        expect(mocks.addTags).toHaveBeenCalledWith({
            userId: 'usr_self',
            tags: ['language_ja']
        });
        expect(
            mocks.updateCurrentUser.mock.invocationCallOrder[0]
        ).toBeLessThan(mocks.removeTags.mock.invocationCallOrder[0]);
        expect(mocks.removeTags.mock.invocationCallOrder[0]).toBeLessThan(
            mocks.addTags.mock.invocationCallOrder[0]
        );
        expect(rendered.result.current.profileDetailsDialog.open).toBe(false);
        expect(rendered.actionStatusRef.current).toBe('idle');
    });
});
