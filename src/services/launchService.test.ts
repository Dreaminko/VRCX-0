import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getInstanceShortName: vi.fn(),
    resolveVrcLaunchUrl: vi.fn(),
    joinInstanceWithFallback: vi.fn(),
    sendSelfInviteToInstance: vi.fn()
}));

vi.mock('sonner', () => ({
    toast: {
        success: vi.fn(),
        warning: vi.fn()
    }
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {}
}));

vi.mock('@/repositories/configRepository', () => ({
    default: {}
}));

vi.mock('@/repositories/vrchatInstanceRepository', () => ({
    default: {
        getInstanceShortName: mocks.getInstanceShortName
    }
}));

vi.mock('@/services/directAccessService', () => ({
    resolveVrcLaunchUrl: mocks.resolveVrcLaunchUrl
}));

vi.mock('@/services/hostCapabilityService', () => ({
    requireHostCapabilitySupported: vi.fn()
}));

vi.mock('@/services/i18nService', () => ({
    default: {
        t: (key: string) => key
    }
}));

vi.mock('@/services/instanceActionService', () => ({
    joinInstanceWithFallback: mocks.joinInstanceWithFallback,
    sendSelfInviteToInstance: mocks.sendSelfInviteToInstance
}));

import {
    attachRunningVrchat,
    resolveLaunchDialogDetails,
    selfInviteToInstance
} from './launchService';

const WORLD_ID = 'wrld_12345678-1234-1234-1234-1234567890ab';
const INSTANCE_ID = '12345~hidden(usr_owner)~region(jp)';
const LOCATION = `${WORLD_ID}:${INSTANCE_ID}`;

describe('launchService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.joinInstanceWithFallback.mockResolvedValue({ status: 'opened' });
        mocks.resolveVrcLaunchUrl.mockResolvedValue(
            `vrchat://launch?id=${LOCATION}`
        );
    });

    it('uses the canonical instance location when attaching from a web launch URL', async () => {
        const launchUrl = `https://vrchat.com/home/launch?worldId=${WORLD_ID}&instanceId=${encodeURIComponent(INSTANCE_ID)}&shortName=freshTok`;

        await attachRunningVrchat(launchUrl);

        expect(mocks.joinInstanceWithFallback).toHaveBeenCalledWith(
            LOCATION,
            'freshTok'
        );
    });

    it('uses the canonical instance location when self-inviting from a vrchat launch URL', async () => {
        const launchUrl = `vrchat://launch?id=${encodeURIComponent(LOCATION)}&shortName=freshTok`;

        await selfInviteToInstance(launchUrl);

        expect(mocks.sendSelfInviteToInstance).toHaveBeenCalledWith(
            LOCATION,
            'freshTok'
        );
    });

    it('keeps location-only launch details when short-name resolution fails', async () => {
        mocks.getInstanceShortName.mockRejectedValue(
            new Error('short-name service unavailable')
        );

        await expect(
            resolveLaunchDialogDetails(LOCATION)
        ).resolves.toMatchObject({
            tag: LOCATION,
            location: LOCATION,
            shortName: '',
            launchToken: '',
            vrcUrl: `vrchat://launch?id=${LOCATION}`
        });
    });

    it('resolves details for a web launch URL instead of treating the URL as an instance tag', async () => {
        const launchUrl = `https://vrchat.com/home/launch?worldId=${WORLD_ID}&instanceId=${encodeURIComponent(INSTANCE_ID)}&shortName=freshTok`;
        mocks.resolveVrcLaunchUrl.mockResolvedValue(
            `vrchat://launch?id=${LOCATION}&shortName=freshTok`
        );

        await expect(
            resolveLaunchDialogDetails(launchUrl)
        ).resolves.toMatchObject({
            tag: launchUrl,
            location: LOCATION,
            shortName: 'freshTok',
            launchToken: 'freshTok'
        });
    });
});
