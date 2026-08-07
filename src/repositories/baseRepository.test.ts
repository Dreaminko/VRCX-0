import { describe, expect, it } from 'vitest';

import {
    asString,
    createKeyPrefixer,
    safeJsonParse,
    safeJsonStringify
} from './baseRepository';

describe('asString', () => {
    it('falls back to the caller-supplied default when a preference was never stored, instead of showing "undefined" in the UI', () => {
        expect(asString(undefined, 'default-value')).toBe('default-value');
        expect(asString(null, 'default-value')).toBe('default-value');
    });

    it('preserves a stored value of "0" or "false" rather than treating it as missing', () => {
        expect(asString(0, 'fallback')).toBe('0');
        expect(asString(false, 'fallback')).toBe('false');
        expect(asString('', 'fallback')).toBe('');
    });
});

describe('safeJsonParse', () => {
    it('restores a previously saved preference object', () => {
        expect(safeJsonParse('{"enabled":true,"count":3}')).toEqual({
            enabled: true,
            count: 3
        });
    });

    it('does not crash app startup when a preference file was corrupted on disk, falling back instead', () => {
        expect(safeJsonParse('{not valid json', 'fallback')).toBe('fallback');
        expect(safeJsonParse('{not valid json')).toBeNull();
    });

    it('treats a key that was never written as absent rather than as a parse failure', () => {
        expect(safeJsonParse(undefined, 'fallback')).toBe('fallback');
        expect(safeJsonParse(null, 'fallback')).toBe('fallback');
        expect(safeJsonParse('', 'fallback')).toBe('fallback');
    });
});

describe('safeJsonStringify', () => {
    it('serializes a preference object for persistence', () => {
        expect(safeJsonStringify({ enabled: true, count: 3 })).toBe(
            '{"enabled":true,"count":3}'
        );
    });

    it('never throws when asked to persist a value JSON cannot represent, so a save action can never crash the app', () => {
        const circular: Record<string, unknown> = {};
        circular.self = circular;

        expect(safeJsonStringify(circular)).toBe('null');
        expect(safeJsonStringify(10n)).toBe('null');
    });

    it('round-trips through safeJsonParse without losing data', () => {
        const original = { providerList: ['a', 'b'], selected: 'a' };
        expect(safeJsonParse(safeJsonStringify(original))).toEqual(original);
    });
});

describe('createKeyPrefixer', () => {
    it('namespaces keys so two repositories sharing one key-value store cannot collide', () => {
        const configKey = createKeyPrefixer('config:');
        const storageKey = createKeyPrefixer('storage:');

        expect(configKey('theme')).toBe('config:theme');
        expect(storageKey('theme')).toBe('storage:theme');
        expect(configKey('theme')).not.toBe(storageKey('theme'));
    });
});
