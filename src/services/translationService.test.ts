import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getBool: vi.fn(),
    getString: vi.fn(),
    appTranslationTranslate: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appTranslationTranslate: mocks.appTranslationTranslate
    }
}));

vi.mock('@/repositories/configRepository', () => ({
    default: {
        getBool: mocks.getBool,
        getString: mocks.getString
    }
}));

import {
    getTranslationConfig,
    translateText,
    translateTextDetailed
} from './translationService';

describe('translationService', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        mocks.getBool.mockResolvedValue(true);
        mocks.getString.mockImplementation((key: string, fallback = '') => {
            const values: Record<string, string> = {
                bioLanguage: 'ja',
                translationAPIType: 'deepl'
            };
            return Promise.resolve(values[key] ?? String(fallback ?? ''));
        });
    });

    it('reads the display config from the config repository', async () => {
        const config = await getTranslationConfig();
        expect(config).toEqual({
            enabled: true,
            bioLanguage: 'ja',
            type: 'deepl'
        });
    });

    it('delegates translation to the runtime command', async () => {
        mocks.appTranslationTranslate.mockResolvedValue({
            text: 'こんにちは',
            detectedSourceLanguage: 'en',
            provider: 'deepl'
        });

        const result = await translateTextDetailed('Hello', 'ja');

        expect(mocks.appTranslationTranslate).toHaveBeenCalledWith({
            text: 'Hello',
            targetLanguage: 'ja',
            overrides: null
        });
        expect(result).toEqual({
            text: 'こんにちは',
            detectedSourceLang: 'en'
        });
    });

    it('passes a null target language when none is provided', async () => {
        mocks.appTranslationTranslate.mockResolvedValue({
            text: 'hola',
            detectedSourceLanguage: null,
            provider: 'google'
        });

        await expect(translateText('Hello')).resolves.toBe('hola');
        expect(mocks.appTranslationTranslate).toHaveBeenCalledWith({
            text: 'Hello',
            targetLanguage: null,
            overrides: null
        });
    });

    it('propagates command failures to the caller', async () => {
        mocks.appTranslationTranslate.mockRejectedValue(
            new Error('Translation API disabled.')
        );

        await expect(translateText('Hello')).rejects.toThrow(
            'Translation API disabled.'
        );
    });
});
