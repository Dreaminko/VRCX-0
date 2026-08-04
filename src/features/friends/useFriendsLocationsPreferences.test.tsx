// @vitest-environment jsdom

import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    boolValues: new Map<string, boolean>(),
    stringValues: new Map<string, string>(),
    getBool: vi.fn(),
    getString: vi.fn(),
    setBool: vi.fn(),
    setString: vi.fn()
}));

vi.mock('@/repositories/configRepository', () => ({
    default: {
        getBool: mocks.getBool,
        getString: mocks.getString,
        setBool: mocks.setBool,
        setString: mocks.setString
    }
}));

import { publishPreferenceChanged } from '@/shared/events/preferenceEvents';

import { useFriendsLocationsPreferences } from './useFriendsLocationsPreferences';

describe('useFriendsLocationsPreferences', () => {
    beforeEach(() => {
        mocks.boolValues.clear();
        mocks.stringValues.clear();
        mocks.getBool
            .mockReset()
            .mockImplementation(
                async (key: string, fallback = false) =>
                    mocks.boolValues.get(key) ?? fallback
            );
        mocks.getString
            .mockReset()
            .mockImplementation(
                async (key: string, fallback = '') =>
                    mocks.stringValues.get(key) ?? String(fallback)
            );
        mocks.setBool.mockReset().mockResolvedValue(undefined);
        mocks.setString.mockReset().mockResolvedValue(undefined);
    });

    it('tracks the sidebar current-user visibility preference', async () => {
        mocks.boolValues.set('isShowCurrentUserInSameInstance', true);
        const { result } = renderHook(() => useFriendsLocationsPreferences());

        await waitFor(() => expect(result.current.preferencesReady).toBe(true));
        expect(result.current.showCurrentUserInSameInstance).toBe(true);

        mocks.boolValues.set('isShowCurrentUserInSameInstance', false);
        act(() => {
            publishPreferenceChanged('isShowCurrentUserInSameInstance', false);
        });

        await waitFor(() =>
            expect(result.current.showCurrentUserInSameInstance).toBe(false)
        );
    });
});
