// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from 'vitest';

import {
    normalizePreferenceKey,
    onPreferenceChanged,
    publishPreferenceChanged
} from './preferenceEvents';

describe('normalizePreferenceKey', () => {
    it('strips the storage-layer "VRCX_" prefix, so feature hooks can subscribe by the short preference name regardless of how it is namespaced on disk', () => {
        expect(normalizePreferenceKey('VRCX_avatarRemoteDatabase')).toBe(
            'avatarRemoteDatabase'
        );
    });

    it('leaves a key with no storage prefix unchanged', () => {
        expect(normalizePreferenceKey('feedTimeDisplayMode')).toBe(
            'feedTimeDisplayMode'
        );
    });
});

describe('onPreferenceChanged / publishPreferenceChanged', () => {
    afterEach(() => {
        vi.restoreAllMocks();
    });

    it('notifies a subscriber when the preference it is watching changes, even if the writer used the prefixed storage key', () => {
        const onFeedTimeDisplayModeChanged = vi.fn();
        const unsubscribe = onPreferenceChanged(
            'feedTimeDisplayMode',
            onFeedTimeDisplayModeChanged
        );

        publishPreferenceChanged('VRCX_feedTimeDisplayMode', 'relative');

        expect(onFeedTimeDisplayModeChanged).toHaveBeenCalledWith(
            'relative',
            expect.objectContaining({
                normalizedKey: 'feedTimeDisplayMode'
            })
        );

        unsubscribe();
    });

    it('ignores changes to preferences the subscriber did not ask about, so unrelated settings updates cannot trigger its callback', () => {
        const onAvatarDatabaseChanged = vi.fn();
        const unsubscribe = onPreferenceChanged(
            'avatarRemoteDatabase',
            onAvatarDatabaseChanged
        );

        publishPreferenceChanged('VRCX_feedTimeDisplayMode', 'relative');

        expect(onAvatarDatabaseChanged).not.toHaveBeenCalled();

        unsubscribe();
    });

    it('supports watching several preference keys from one subscription, useful for a settings panel reacting to any of a related group', () => {
        const onEitherChanged = vi.fn();
        const unsubscribe = onPreferenceChanged(
            ['avatarRemoteDatabase', 'feedTimeDisplayMode'],
            onEitherChanged
        );

        publishPreferenceChanged('VRCX_avatarRemoteDatabase', true);
        publishPreferenceChanged('VRCX_feedTimeDisplayMode', 'absolute');

        expect(onEitherChanged).toHaveBeenCalledTimes(2);

        unsubscribe();
    });

    it('stops delivering events once the caller unsubscribes, preventing updates on an unmounted component', () => {
        const onChanged = vi.fn();
        const unsubscribe = onPreferenceChanged(
            'avatarRemoteDatabase',
            onChanged
        );
        unsubscribe();

        publishPreferenceChanged('VRCX_avatarRemoteDatabase', true);

        expect(onChanged).not.toHaveBeenCalled();
    });
});
