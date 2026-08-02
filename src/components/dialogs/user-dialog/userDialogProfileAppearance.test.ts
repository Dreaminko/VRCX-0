import { describe, expect, it } from 'vitest';

import {
    mergeUserDialogProfileAppearance,
    normalizeProfileAppearanceColor,
    preserveUserDialogProfileAppearance,
    resolveProfileDecorationAssetUrls,
    resolveUserDialogBannerUrl
} from './userDialogProfileAppearance';

describe('mergeUserDialogProfileAppearance', () => {
    it('merges only appearance fields and preserves explicit empty values', () => {
        const user = {
            id: 'usr_target',
            displayName: 'Ordinary user',
            status: 'active',
            location: 'wrld_live:instance',
            iconFrame: 'invt_old'
        };

        expect(
            mergeUserDialogProfileAppearance(
                user,
                {
                    id: 'usr_target',
                    displayName: 'Profile endpoint name',
                    status: 'offline',
                    location: 'offline',
                    iconFrame: '',
                    profileEffect: 'invt_profile',
                    bannerColor: '2cc968'
                },
                'usr_target'
            )
        ).toEqual({
            ...user,
            iconFrame: '',
            profileEffect: 'invt_profile',
            bannerColor: '2cc968'
        });
    });

    it('does not clear fields omitted by the profile endpoint', () => {
        const user = {
            id: 'usr_target',
            iconFrame: 'invt_frame',
            profileEffect: 'invt_profile'
        };

        expect(
            mergeUserDialogProfileAppearance(
                user,
                {
                    id: 'usr_target',
                    iconFrame: ''
                },
                'usr_target'
            )
        ).toEqual({
            id: 'usr_target',
            iconFrame: '',
            profileEffect: 'invt_profile'
        });
    });

    it('ignores a profile response for another target', () => {
        const user = {
            id: 'usr_target',
            iconFrame: 'invt_frame'
        };

        expect(
            mergeUserDialogProfileAppearance(
                user,
                {
                    id: 'usr_other',
                    iconFrame: 'invt_other'
                },
                'usr_target'
            )
        ).toBe(user);
    });
});

describe('preserveUserDialogProfileAppearance', () => {
    it('keeps profile-only background fields after an ordinary user update', () => {
        expect(
            preserveUserDialogProfileAppearance(
                {
                    id: 'usr_target',
                    displayName: 'Updated user',
                    bio: 'Updated bio'
                },
                {
                    id: 'usr_target',
                    displayName: 'Target',
                    backgroundType: 'texture',
                    backgroundTextureId: 'grid',
                    bannerType: 'customImage',
                    bannerCustomUrl: 'https://example.test/banner.png'
                }
            )
        ).toEqual({
            id: 'usr_target',
            displayName: 'Updated user',
            bio: 'Updated bio',
            backgroundType: 'texture',
            backgroundTextureId: 'grid',
            bannerType: 'customImage',
            bannerCustomUrl: 'https://example.test/banner.png'
        });
    });

    it('keeps explicit appearance values from the update response', () => {
        expect(
            preserveUserDialogProfileAppearance(
                {
                    id: 'usr_target',
                    userIcon: ''
                },
                {
                    id: 'usr_target',
                    userIcon: 'https://example.test/old-icon.png',
                    profileEffect: 'invt_profile'
                }
            )
        ).toEqual({
            id: 'usr_target',
            userIcon: '',
            profileEffect: 'invt_profile'
        });
    });
});

describe('profile appearance assets', () => {
    const item = {
        id: 'invt_profile',
        metadata: {
            assets: [
                {
                    type: 'introAnimation',
                    url: 'https://example.test/intro.webp'
                },
                {
                    type: 'base',
                    url: 'https://example.test/base.webp'
                },
                {
                    type: 'mainAnimation',
                    url: 'https://example.test/main.webp'
                }
            ]
        }
    };

    it('uses the looping animation normally and the base asset for reduced motion', () => {
        expect(resolveProfileDecorationAssetUrls(item)).toEqual({
            animatedUrl: 'https://example.test/main.webp',
            staticUrl: 'https://example.test/base.webp'
        });
    });

    it('does not use intro animations or inventory thumbnails as a persistent effect', () => {
        expect(
            resolveProfileDecorationAssetUrls({
                id: 'invt_intro_only',
                imageUrl: 'https://example.test/thumbnail.png',
                metadata: {
                    assets: [
                        {
                            type: 'introAnimation',
                            url: 'https://example.test/intro.webp'
                        }
                    ]
                }
            })
        ).toEqual({
            animatedUrl: '',
            staticUrl: ''
        });
    });

    it('accepts six-digit colors without inventing styles from ids', () => {
        expect(normalizeProfileAppearanceColor('2CC968')).toBe('#2cc968');
        expect(normalizeProfileAppearanceColor('theme_default')).toBe('');
        expect(normalizeProfileAppearanceColor('')).toBe('');
    });

    it('ignores retained image urls for color banners', () => {
        expect(
            resolveUserDialogBannerUrl({
                bannerType: 'color',
                bannerUrl: 'https://example.test/old-banner.png',
                bannerCustomUrl: 'https://example.test/old-custom.png'
            })
        ).toBe('');
    });

    it('prefers the resolved banner url for image banners', () => {
        expect(
            resolveUserDialogBannerUrl({
                bannerType: 'customImage',
                bannerUrl: 'https://example.test/banner.png',
                bannerCustomUrl: 'https://example.test/custom.png'
            })
        ).toBe('https://example.test/banner.png');
    });

    it('returns no profile banner when image urls are empty', () => {
        expect(
            resolveUserDialogBannerUrl({
                bannerType: 'customImage',
                bannerUrl: '',
                bannerCustomUrl: ''
            })
        ).toBe('');
    });
});
