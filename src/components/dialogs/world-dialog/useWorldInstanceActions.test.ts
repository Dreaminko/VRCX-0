// @vitest-environment jsdom

import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    createInstance: vi.fn(),
    resolveCreatedInstanceDetails: vi.fn(),
    selfInviteToInstance: vi.fn(),
    tryOpenLaunchLocation: vi.fn(),
    showLaunchDialog: vi.fn()
}));

vi.mock('react-i18next', async (importOriginal) => {
    const actual = await importOriginal<typeof import('react-i18next')>();
    return {
        ...actual,
        useTranslation: () => ({ t: (key: string) => key })
    };
});

vi.mock('sonner', () => ({
    toast: {
        success: vi.fn(),
        error: vi.fn(),
        warning: vi.fn()
    }
}));

vi.mock('@/repositories/configRepository', () => ({
    default: {
        setString: vi.fn().mockResolvedValue(undefined),
        setBool: vi.fn().mockResolvedValue(undefined),
        getString: vi.fn().mockResolvedValue(''),
        getBool: vi.fn().mockResolvedValue(false)
    }
}));

vi.mock('@/repositories/vrchatInstanceRepository', () => ({
    default: { createInstance: mocks.createInstance }
}));

vi.mock('@/services/launchService', () => ({
    selfInviteToInstance: mocks.selfInviteToInstance
}));

vi.mock('@/services/directAccessService', () => ({
    tryOpenLaunchLocation: mocks.tryOpenLaunchLocation
}));

vi.mock('./worldInstanceResolver', () => ({
    resolveCreatedInstanceDetails: mocks.resolveCreatedInstanceDetails
}));

import worldProfileRepository from '@/repositories/worldProfileRepository';

import {
    isNewInstanceOpenInGameRequest,
    isNewInstanceSelfInviteRequest,
    resolveNewInstanceAfterCreateAction,
    useWorldInstanceActions
} from './useWorldInstanceActions';
import type { NewInstanceAfterCreateAction } from './worldNewInstanceTypes';

describe('useWorldInstanceActions helpers', () => {
    it('maps the follow-up new-instance action to open in-game while VRChat is running', () => {
        expect(resolveNewInstanceAfterCreateAction(true, true)).toBe(
            'openInGame'
        );
        expect(
            isNewInstanceOpenInGameRequest({ afterCreateAction: 'openInGame' })
        ).toBe(true);
        expect(
            isNewInstanceSelfInviteRequest({ afterCreateAction: 'openInGame' })
        ).toBe(false);
    });

    it('keeps the follow-up new-instance action as self-invite when VRChat is not running', () => {
        expect(resolveNewInstanceAfterCreateAction(true, false)).toBe(
            'selfInvite'
        );
        expect(
            isNewInstanceSelfInviteRequest({ afterCreateAction: 'selfInvite' })
        ).toBe(true);
        expect(
            isNewInstanceOpenInGameRequest({ afterCreateAction: 'selfInvite' })
        ).toBe(false);
    });

    it('does not attach a follow-up action for a plain new instance', () => {
        expect(resolveNewInstanceAfterCreateAction(false, true)).toBe('');
        expect(isNewInstanceSelfInviteRequest(null)).toBe(false);
        expect(isNewInstanceOpenInGameRequest({})).toBe(false);
    });
});

const created = {
    location: 'wrld_test:12345',
    shortName: 'shrt',
    secureOrShortName: 'shrt',
    url: 'https://vrchat.com/home/launch?worldId=wrld_test',
    accessType: 'public',
    ownerId: 'usr_self',
    groupId: '',
    group: null
};

function renderCreateFlow(
    afterCreateAction: NewInstanceAfterCreateAction,
    isGameRunning = false
) {
    const actionStatusRef = { current: 'idle' };
    const { result } = renderHook(() =>
        useWorldInstanceActions({
            world: worldProfileRepository.normalize({
                id: 'wrld_test',
                name: 'Test World'
            }),
            currentEndpoint: 'endpoint-a',
            currentUserId: 'usr_self',
            isGameRunning,
            profileWorldId: 'wrld_test',
            newInstanceGroups: [],
            actionStatusRef,
            setActionStatus: vi.fn(),
            isCurrentWorldTarget: () => true,
            showLaunchDialog: mocks.showLaunchDialog
        })
    );
    act(() => {
        result.current.setNewInstanceRequest({
            selfInvite: afterCreateAction === 'selfInvite',
            afterCreateAction,
            defaults: {}
        });
    });
    return result;
}

async function submit(result: {
    current: ReturnType<typeof useWorldInstanceActions>;
}) {
    await act(async () => {
        await result.current.createWorldInstance({
            selectedTab: 'Normal',
            accessType: 'public',
            region: 'US West',
            groupId: '',
            groupAccessType: 'plus',
            queueEnabled: true,
            ageGate: false,
            displayName: '',
            displayNamePresets: [],
            roleIds: '',
            instanceName: '',
            legacyUserId: '',
            strict: false
        });
    });
}

describe('useWorldInstanceActions createWorldInstance', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.createInstance.mockResolvedValue({
            json: { location: created.location }
        });
        mocks.resolveCreatedInstanceDetails.mockResolvedValue(created);
        mocks.selfInviteToInstance.mockResolvedValue(undefined);
        mocks.tryOpenLaunchLocation.mockResolvedValue(true);
    });

    it('closes the dialog and hands a plain new instance to the launch dialog', async () => {
        const result = renderCreateFlow('');
        await submit(result);

        expect(result.current.newInstanceRequest).toBeNull();
        expect(mocks.showLaunchDialog).toHaveBeenCalledWith(
            created.location,
            created.shortName,
            created.secureOrShortName,
            expect.objectContaining({ createdInstance: created })
        );
    });

    it('closes the dialog without the launch dialog when the follow-up self-invite succeeds', async () => {
        const result = renderCreateFlow('selfInvite');
        await submit(result);

        expect(result.current.newInstanceRequest).toBeNull();
        expect(mocks.selfInviteToInstance).toHaveBeenCalledOnce();
        expect(mocks.showLaunchDialog).not.toHaveBeenCalled();
    });

    it('falls back to the launch dialog when the follow-up self-invite fails', async () => {
        mocks.selfInviteToInstance.mockRejectedValue(new Error('nope'));
        const result = renderCreateFlow('selfInvite');
        await submit(result);

        expect(result.current.newInstanceRequest).toBeNull();
        expect(mocks.showLaunchDialog).toHaveBeenCalledOnce();
    });

    it('closes the dialog without the launch dialog when the follow-up open in-game succeeds', async () => {
        const result = renderCreateFlow('openInGame', true);
        await submit(result);

        expect(result.current.newInstanceRequest).toBeNull();
        expect(mocks.tryOpenLaunchLocation).toHaveBeenCalledOnce();
        expect(mocks.showLaunchDialog).not.toHaveBeenCalled();
    });

    it('falls back to the launch dialog when the follow-up open in-game fails', async () => {
        mocks.tryOpenLaunchLocation.mockRejectedValue(new Error('nope'));
        const result = renderCreateFlow('openInGame', true);
        await submit(result);

        expect(result.current.newInstanceRequest).toBeNull();
        expect(mocks.showLaunchDialog).toHaveBeenCalledOnce();
    });
});
