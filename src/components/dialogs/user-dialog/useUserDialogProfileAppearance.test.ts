// @vitest-environment jsdom

import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getInventoryTemplate: vi.fn()
}));

vi.mock('@/repositories/vrchatMediaRepository', async (importOriginal) => {
    const actual =
        await importOriginal<
            typeof import('@/repositories/vrchatMediaRepository')
        >();
    return {
        ...actual,
        default: {
            ...actual.default,
            getInventoryTemplate: mocks.getInventoryTemplate
        }
    };
});

import { useUserDialogProfileAppearance } from './useUserDialogProfileAppearance';

describe('useUserDialogProfileAppearance', () => {
    beforeEach(() => {
        mocks.getInventoryTemplate.mockReset();
        mocks.getInventoryTemplate.mockImplementation(
            async (inventoryTemplateId: string) => ({
                json: {
                    id: inventoryTemplateId,
                    metadata: {
                        assets: [
                            {
                                type: 'mainAnimation',
                                url: `https://example.test/${inventoryTemplateId}.webp`
                            }
                        ]
                    }
                }
            })
        );
    });

    it('loads the three equipped template ids and maps them to their slots', async () => {
        const { result } = renderHook(() =>
            useUserDialogProfileAppearance({
                profile: {
                    id: 'usr_target',
                    iconFrame: 'invt_frame',
                    profileEffect: 'invt_profile',
                    nameplateEffect: 'invt_nameplate'
                }
            })
        );

        await waitFor(() => {
            expect(result.current.iconFrame?.id).toBe('invt_frame');
            expect(result.current.profileEffect?.id).toBe('invt_profile');
            expect(result.current.nameplateEffect?.id).toBe('invt_nameplate');
        });
        expect(mocks.getInventoryTemplate).toHaveBeenCalledTimes(3);
    });

    it('deduplicates template requests and tolerates one failed decoration', async () => {
        mocks.getInventoryTemplate.mockImplementation(
            async (inventoryTemplateId: string) => {
                if (inventoryTemplateId === 'invt_failed') {
                    throw new Error('not available');
                }
                return {
                    json: {
                        id: inventoryTemplateId
                    }
                };
            }
        );

        const { result } = renderHook(() =>
            useUserDialogProfileAppearance({
                profile: {
                    id: 'usr_target',
                    iconFrame: 'invt_shared',
                    profileEffect: 'invt_failed',
                    nameplateEffect: 'invt_shared'
                }
            })
        );

        await waitFor(() => {
            expect(result.current.iconFrame?.id).toBe('invt_shared');
            expect(result.current.nameplateEffect?.id).toBe('invt_shared');
        });
        expect(result.current.profileEffect).toBeUndefined();
        expect(mocks.getInventoryTemplate).toHaveBeenCalledTimes(2);
    });

    it('does not expose a previous target while the next target is loading', async () => {
        let resolveNext:
            | ((value: { json: { id: string } }) => void)
            | undefined;
        mocks.getInventoryTemplate.mockImplementation(
            (inventoryTemplateId: string) => {
                if (inventoryTemplateId === 'invt_next') {
                    return new Promise<{ json: { id: string } }>((resolve) => {
                        resolveNext = resolve;
                    });
                }
                return Promise.resolve({
                    json: {
                        id: inventoryTemplateId
                    }
                });
            }
        );

        const { result, rerender } = renderHook(
            ({ userId, iconFrame }: { iconFrame: string; userId: string }) =>
                useUserDialogProfileAppearance({
                    profile: {
                        id: userId,
                        iconFrame
                    }
                }),
            {
                initialProps: {
                    userId: 'usr_first',
                    iconFrame: 'invt_first'
                }
            }
        );

        await waitFor(() => {
            expect(result.current.iconFrame?.id).toBe('invt_first');
        });

        rerender({
            userId: 'usr_next',
            iconFrame: 'invt_next'
        });

        expect(result.current.iconFrame).toBeUndefined();
        resolveNext?.({
            json: {
                id: 'invt_next'
            }
        });
        await waitFor(() => {
            expect(result.current.iconFrame?.id).toBe('invt_next');
        });
    });

    it('does not request empty decoration ids', () => {
        const { result } = renderHook(() =>
            useUserDialogProfileAppearance({
                profile: {
                    id: 'usr_target',
                    iconFrame: '',
                    profileEffect: '',
                    nameplateEffect: ''
                }
            })
        );

        expect(result.current).toEqual({});
        expect(mocks.getInventoryTemplate).not.toHaveBeenCalled();
    });
});
