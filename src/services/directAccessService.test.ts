import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    openInstanceInGame: vi.fn(),
    openWorldDialog: vi.fn()
}));

vi.mock('@/repositories/vrchatInstanceRepository', () => ({
    default: {
        getInstanceShortName: vi.fn()
    }
}));

vi.mock('@/repositories/vrchatSearchRepository', () => ({
    default: {}
}));

vi.mock('@/services/dialogService', () => ({
    openAvatarDialog: vi.fn(),
    openGroupDialog: vi.fn(),
    openUserDialog: vi.fn(),
    openWorldDialog: mocks.openWorldDialog
}));

vi.mock('@/services/instanceActionService', () => ({
    openInstanceInGame: mocks.openInstanceInGame
}));

import {
    directAccessParse,
    tryOpenLaunchLocation
} from './directAccessService';

const WORLD_ID = 'wrld_12345678-1234-1234-1234-1234567890ab';
const INSTANCE_ID = '12345~hidden(usr_owner)';
const LOCATION = `${WORLD_ID}:${INSTANCE_ID}`;

describe('directAccessService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('normalizes launch URLs before trying to open the instance', async () => {
        mocks.openInstanceInGame.mockResolvedValue(true);
        const launchUrl = `https://vrchat.com/home/launch?worldId=${WORLD_ID}&instanceId=${encodeURIComponent(INSTANCE_ID)}&shortName=freshTok`;

        await expect(
            tryOpenLaunchLocation(launchUrl, 'freshTok')
        ).resolves.toBe(true);

        expect(mocks.openInstanceInGame).toHaveBeenCalledWith(
            LOCATION,
            'freshTok'
        );
    });

    it('accepts vrchat launch scheme URLs through direct access', async () => {
        mocks.openInstanceInGame.mockResolvedValue(true);
        const launchUrl = `vrchat://launch?id=${encodeURIComponent(LOCATION)}&shortName=freshTok`;

        await expect(directAccessParse(launchUrl)).resolves.toBe(true);

        expect(mocks.openInstanceInGame).toHaveBeenCalledWith(
            LOCATION,
            'freshTok'
        );
        expect(mocks.openWorldDialog).not.toHaveBeenCalled();
    });
});
