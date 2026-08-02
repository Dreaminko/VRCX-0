import { describe, expect, it } from 'vitest';

import {
    buildGradientBackgroundUpdate,
    isProfileBackgroundTextureAvailable,
    profileBackgroundTextureLabel,
    resolveProfileGradientColors
} from './profileBackgroundSelection';

describe('profile background selection', () => {
    it('keeps Grid available without VRC+ and gates other textures', () => {
        expect(isProfileBackgroundTextureAvailable('grid', false)).toBe(true);
        expect(isProfileBackgroundTextureAvailable('cat-dream', false)).toBe(
            false
        );
        expect(isProfileBackgroundTextureAvailable('cat-dream', true)).toBe(
            true
        );
    });

    it('builds the profile gradient payload without CSS hash prefixes', () => {
        expect(buildGradientBackgroundUpdate('#5d3f86', '#21385b')).toEqual({
            backgroundType: 'gradient',
            backgroundGradientTop: '5d3f86',
            backgroundGradientBottom: '21385b'
        });
    });

    it('uses valid profile colors and falls back for missing values', () => {
        expect(
            resolveProfileGradientColors({
                backgroundGradientTop: 'ABCDEF',
                backgroundGradientBottom: '123456'
            })
        ).toEqual({ top: '#abcdef', bottom: '#123456' });
        expect(resolveProfileGradientColors({})).toEqual({
            top: '#5d3f86',
            bottom: '#21385b'
        });
    });

    it('turns texture ids into their official display names', () => {
        expect(profileBackgroundTextureLabel('cat-dream')).toBe('Cat Dream');
        expect(profileBackgroundTextureLabel('i-am-speed')).toBe('I Am Speed');
    });
});
