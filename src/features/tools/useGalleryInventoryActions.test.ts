import { describe, expect, it, vi } from 'vitest';

import { useGalleryInventoryActions } from './useGalleryInventoryActions';

describe('useGalleryInventoryActions', () => {
    it('allows a non-VRC+ user to set profile icons and banners', async () => {
        const nextUser = {
            id: 'usr_self',
            displayName: 'Current User',
            userIcon: 'https://api.vrchat.cloud/api/1/file/file_icon/1'
        };
        const updateCurrentUser = vi.fn().mockResolvedValue(nextUser);
        const setAuthBootstrap = vi.fn();
        const toast = {
            error: vi.fn(),
            success: vi.fn()
        };
        const currentUserSnapshot = {
            id: 'usr_self',
            $isVRCPlus: false,
            tags: []
        };
        const actions = useGalleryInventoryActions({
            buildProfilePicOverride: (endpoint: string, fileId: string) =>
                `${endpoint}/file/${fileId}/1`,
            currentUserProfileService: {
                updateCurrentUser
            },
            currentEndpoint: 'https://api.vrchat.cloud/api/1',
            currentUserId: 'usr_self',
            currentUserSnapshot,
            confirm: vi.fn(),
            getAuthTarget: () => ({
                userId: 'usr_self',
                endpoint: 'https://api.vrchat.cloud/api/1'
            }),
            isRuntimeAuthTarget: () => true,
            mediaRepository: {
                consumeInventoryBundle: vi.fn(),
                deletePrint: vi.fn(),
                redeemReward: vi.fn(),
                setPrintFavorite: vi.fn()
            },
            prompt: vi.fn(),
            refreshInventory: vi.fn(),
            setAssets: vi.fn(),
            setMutatingKey: vi.fn(),
            t: (key: string) => key,
            toast,
            useRuntimeStore: {
                getState: () => ({
                    auth: {
                        currentUserSnapshot
                    },
                    setAuthBootstrap
                })
            }
        });

        await actions.setProfileField('userIcon', 'file_icon');

        expect(updateCurrentUser).toHaveBeenCalledWith({
            userId: 'usr_self',
            params: {
                userIcon: 'https://api.vrchat.cloud/api/1/file/file_icon/1'
            }
        });
        expect(setAuthBootstrap).toHaveBeenCalledWith({
            currentUserSnapshot: nextUser,
            currentUserDisplayName: 'Current User'
        });
        expect(toast.error).not.toHaveBeenCalled();
        expect(toast.success).toHaveBeenCalledWith(
            'message.gallery.profile_icon_changed'
        );

        await actions.setProfileField('profilePicOverride', 'file_banner');

        expect(updateCurrentUser).toHaveBeenLastCalledWith({
            userId: 'usr_self',
            params: {
                profilePicOverride:
                    'https://api.vrchat.cloud/api/1/file/file_banner/1'
            }
        });
        expect(toast.success).toHaveBeenLastCalledWith(
            'message.gallery.profile_pic_changed'
        );
    });
});
