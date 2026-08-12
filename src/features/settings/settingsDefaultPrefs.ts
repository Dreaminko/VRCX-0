import {
    DEFAULT_PREFERENCES,
    normalizePreferenceSnapshot
} from '@/state/preferencesStore';

export function createDefaultSettingsPrefs() {
    return normalizePreferenceSnapshot(DEFAULT_PREFERENCES);
}
