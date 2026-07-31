import {
    commands,
    type TranslationOverrides,
    type TranslationResult
} from '@/platform/tauri/bindings';
import configRepository from '@/repositories/configRepository';
import {
    normalizeTranslationApiType,
    type TranslationApiType
} from '@/state/preferencesStore';

type TranslationConfig = {
    enabled: boolean;
    bioLanguage: string;
    type: TranslationApiType;
};

export async function getTranslationConfig(): Promise<TranslationConfig> {
    const [enabled, bioLanguage, type] = await Promise.all([
        configRepository.getBool('translationAPI', false),
        configRepository.getString('bioLanguage', 'en'),
        configRepository.getString('translationAPIType', 'google')
    ]);

    return {
        enabled: Boolean(enabled),
        bioLanguage: String(bioLanguage || 'en'),
        type: normalizeTranslationApiType(type)
    };
}

export type TranslationDetailedResult = {
    text: string;
    detectedSourceLang: string | null;
};

export async function translateTextDetailed(
    text: string,
    targetLanguage: unknown = '',
    overrides: TranslationOverrides | null = null
): Promise<TranslationDetailedResult> {
    const result: TranslationResult = await commands.appTranslationTranslate({
        text,
        targetLanguage: String(targetLanguage || '') || null,
        overrides
    });
    return {
        text: result.text,
        detectedSourceLang: result.detectedSourceLanguage
    };
}

export async function translateText(
    text: string,
    targetLanguage: unknown = ''
): Promise<string> {
    const result = await translateTextDetailed(text, targetLanguage);
    return result.text;
}
