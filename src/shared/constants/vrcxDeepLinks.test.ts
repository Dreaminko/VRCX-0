import { describe, expect, it } from 'vitest';

import {
    VRCX_OPEN_RELAY_ORIGIN,
    vrcxAvatarDeepLink,
    vrcxWorldDeepLink
} from './vrcxDeepLinks';

const UUID = '12345678-1234-1234-1234-1234567890ab';

describe('vrcxDeepLinks', () => {
    it('builds canonical world and avatar relay links', () => {
        expect(vrcxWorldDeepLink(`wrld_${UUID}`)).toBe(
            `${VRCX_OPEN_RELAY_ORIGIN}/world/wrld_${UUID}`
        );
        expect(vrcxAvatarDeepLink(`avtr_${UUID}`)).toBe(
            `${VRCX_OPEN_RELAY_ORIGIN}/avatar/avtr_${UUID}`
        );
    });

    it('normalizes surrounding whitespace and rejects invalid ids', () => {
        expect(vrcxWorldDeepLink(` wrld_${UUID} `)).toBe(
            `${VRCX_OPEN_RELAY_ORIGIN}/world/wrld_${UUID}`
        );
        expect(vrcxWorldDeepLink(`avtr_${UUID}`)).toBe('');
        expect(vrcxAvatarDeepLink('avtr_invalid')).toBe('');
    });
});
