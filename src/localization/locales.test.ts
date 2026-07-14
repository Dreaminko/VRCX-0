import { describe, expect, it } from 'vitest';

import cs from './cs.json';
import de from './de.json';
import en from './en.json';
import es from './es.json';
import fr from './fr.json';
import hu from './hu.json';
import ja from './ja.json';
import ko from './ko.json';
import localeCases from './locale-cases.json';
import { languageCodes, normalizeLanguageCode } from './locales';
import pl from './pl.json';
import pt from './pt.json';
import ru from './ru.json';
import th from './th.json';
import vi from './vi.json';
import zhCn from './zh-CN.json';
import zhTw from './zh-TW.json';

const localeSources: Record<string, unknown> = {
    cs,
    de,
    en,
    es,
    fr,
    hu,
    ja,
    ko,
    pl,
    pt,
    ru,
    th,
    vi,
    'zh-CN': zhCn,
    'zh-TW': zhTw
};

describe('normalizeLanguageCode', () => {
    it('keeps exact supported language codes', () => {
        expect(normalizeLanguageCode('en')).toBe('en');
        expect(normalizeLanguageCode('ja')).toBe('ja');
        expect(normalizeLanguageCode('zh-CN')).toBe('zh-CN');
        expect(normalizeLanguageCode('zh-TW')).toBe('zh-TW');
    });

    it('maps regional system languages to supported app languages', () => {
        expect(normalizeLanguageCode('en-US')).toBe('en');
        expect(normalizeLanguageCode('ja-JP')).toBe('ja');
        expect(normalizeLanguageCode('ko-KR')).toBe('ko');
        expect(normalizeLanguageCode('pt-BR')).toBe('pt');
    });

    it('normalizes underscore separators from host locale values', () => {
        expect(normalizeLanguageCode('en_US')).toBe('en');
        expect(normalizeLanguageCode('zh_Hant_TW')).toBe('zh-TW');
    });

    it('maps simplified and traditional Chinese system locales explicitly', () => {
        expect(normalizeLanguageCode('zh')).toBe('zh-CN');
        expect(normalizeLanguageCode('zh-Hans')).toBe('zh-CN');
        expect(normalizeLanguageCode('zh-Hans-CN')).toBe('zh-CN');
        expect(normalizeLanguageCode('zh-SG')).toBe('zh-CN');
        expect(normalizeLanguageCode('zh-Hant')).toBe('zh-TW');
        expect(normalizeLanguageCode('zh-Hant-HK')).toBe('zh-TW');
        expect(normalizeLanguageCode('zh-HK')).toBe('zh-TW');
    });

    it('maps regional German system locale to the supported app language', () => {
        expect(normalizeLanguageCode('de-DE')).toBe('de');
    });

    it('falls back to English for unsupported or empty languages', () => {
        expect(normalizeLanguageCode('xx-XX')).toBe('en');
        expect(normalizeLanguageCode('')).toBe('en');
        expect(normalizeLanguageCode(null)).toBe('en');
    });

    it('matches the shared Rust normalization cases', () => {
        for (const localeCase of localeCases) {
            expect(normalizeLanguageCode(localeCase.input)).toBe(
                localeCase.expected
            );
        }
    });
});

describe('native shell locale coverage', () => {
    const requiredMenuKeys = [
        'nativeShell.menu.app.title',
        'nativeShell.menu.app.about',
        'nativeShell.menu.app.settings',
        'nativeShell.menu.app.checkUpdates',
        'nativeShell.menu.app.restart',
        'nativeShell.menu.app.startBackgroundMode',
        'nativeShell.menu.app.logout',
        'nativeShell.menu.app.quit',
        'nativeShell.menu.view.title',
        'nativeShell.menu.view.notificationCenter',
        'nativeShell.menu.view.quickSearch',
        'nativeShell.menu.view.directAccess',
        'nativeShell.menu.view.toggleNav',
        'nativeShell.menu.view.toggleFriendsSidebar',
        'nativeShell.menu.view.customNav',
        'nativeShell.menu.view.themes',
        'nativeShell.menu.view.zoomIn',
        'nativeShell.menu.view.zoomOut',
        'nativeShell.menu.view.resetZoom',
        'nativeShell.menu.edit.title',
        'nativeShell.menu.edit.undo',
        'nativeShell.menu.edit.redo',
        'nativeShell.menu.edit.cut',
        'nativeShell.menu.edit.copy',
        'nativeShell.menu.edit.paste',
        'nativeShell.menu.edit.selectAll',
        'nativeShell.menu.tools.title',
        'nativeShell.menu.tools.allTools',
        'nativeShell.menu.window.title',
        'nativeShell.menu.window.minimize',
        'nativeShell.menu.window.maximize',
        'nativeShell.menu.window.close',
        'nativeShell.menu.help.title',
        'nativeShell.menu.help.changelog',
        'nativeShell.menu.help.keyboardShortcuts',
        'nativeShell.menu.help.reportIssue',
        'nativeShell.menu.help.github',
        'nativeShell.menu.help.discord',
        'nativeShell.menu.help.qqGroup',
        'nativeShell.menu.help.openDevtools',
        'nativeShell.menu.help.supportVrcx'
    ];

    it('keeps native shell menu labels in every locale source file', () => {
        for (const locale of languageCodes) {
            const source = readLocaleSource(locale);
            for (const key of requiredMenuKeys) {
                const value = readPath(source, key);
                expect(value, `${locale} ${key}`).toEqual(expect.any(String));
                if (typeof value !== 'string') {
                    continue;
                }
                expect(value.trim()).not.toBe('');
                expect(value).not.toBe(key);
            }
        }
    });
});

describe('settings locale coverage', () => {
    const requiredSettingsKeys = [
        'common.actions.configure',
        'common.actions.reset',
        'view.settings.notifications.notifications.text_to_speech.play',
        'view.settings.notifications.notifications.text_to_speech.tts_test_placeholder',
        'view.settings.notifications.notifications.text_to_speech.tts_enabled_preview',
        'view.settings.notifications.notifications.text_to_speech.tts_voice_preview',
        'view.settings.notifications.notifications.text_to_speech.tts_test_failed'
    ];

    it('keeps notification settings labels in every locale source file', () => {
        for (const locale of languageCodes) {
            const source = readLocaleSource(locale);
            for (const key of requiredSettingsKeys) {
                const value = readPath(source, key);
                expect(value, `${locale} ${key}`).toEqual(expect.any(String));
                if (typeof value !== 'string') {
                    continue;
                }
                expect(value.trim()).not.toBe('');
                expect(value).not.toBe(key);
            }
        }
    });
});

describe('advanced settings locale coverage', () => {
    const advancedUiPrefix = 'view.settings.advanced.advanced_ui';
    const requiredAdvancedKeys = collectStringPaths(
        readPath(en, advancedUiPrefix),
        advancedUiPrefix
    );
    const linkOpeningKeys = [
        'view.settings.advanced.advanced_ui.behavior.deep_link_registration',
        'view.settings.advanced.advanced_ui.behavior.deep_link_registered',
        'view.settings.advanced.advanced_ui.behavior.deep_link_not_registered',
        'view.settings.advanced.advanced_ui.behavior.deep_link_repair',
        'view.settings.advanced.advanced_ui.behavior.deep_link_repair_success',
        'view.settings.advanced.advanced_ui.behavior.deep_link_repair_failed',
        'view.settings.advanced.advanced.launch_commands.header'
    ];
    const technicalDeepLinkTerms: Record<string, RegExp> = {
        cs: /přímých odkazů/i,
        de: /deep[ -]?links?/i,
        en: /deep[ -]?links?/i,
        es: /enlaces profundos/i,
        fr: /liens profonds/i,
        hu: /mélyhivatkoz/i,
        ja: /ディープリンク/,
        ko: /딕\s*링크/,
        pl: /linków bezpośrednich/i,
        pt: /links profundos/i,
        ru: /глубоких ссылок/i,
        th: /ดีปลิงก์/,
        vi: /liên kết sâu/i,
        'zh-CN': /深层链接/,
        'zh-TW': /深層連結/
    };

    it('keeps the reorganized advanced settings localized in every language', () => {
        for (const locale of languageCodes) {
            const source = readLocaleSource(locale);
            for (const key of requiredAdvancedKeys) {
                const value = readPath(source, key);
                expect(value, `${locale} ${key}`).toEqual(expect.any(String));
                if (typeof value !== 'string') {
                    continue;
                }
                expect(value.trim()).not.toBe('');
                if (locale !== 'en') {
                    expect(value, `${locale} ${key}`).not.toBe(
                        readPath(en, key)
                    );
                }
            }
        }
    });

    it('uses plain language for links that open VRCX-0', () => {
        for (const locale of languageCodes) {
            const source = readLocaleSource(locale);
            for (const key of linkOpeningKeys) {
                const value = readPath(source, key);
                if (typeof value !== 'string') {
                    continue;
                }
                expect(value, `${locale} ${key}`).not.toMatch(
                    technicalDeepLinkTerms[locale]
                );
            }
        }
    });
});

describe('profile backup locale coverage', () => {
    const profileBackupPrefix = 'profile_backup';
    const requiredProfileBackupKeys = collectStringPaths(
        readPath(en, profileBackupPrefix),
        profileBackupPrefix
    ).sort();

    it('keeps every backup and restore label in all 15 locales', () => {
        for (const locale of languageCodes) {
            const source = readLocaleSource(locale);
            const localizedKeys = collectStringPaths(
                readPath(source, profileBackupPrefix),
                profileBackupPrefix
            ).sort();
            expect(localizedKeys, locale).toEqual(requiredProfileBackupKeys);

            for (const key of requiredProfileBackupKeys) {
                const value = readPath(source, key);
                expect(value, `${locale} ${key}`).toEqual(expect.any(String));
                if (typeof value !== 'string') {
                    continue;
                }
                expect(value.trim()).not.toBe('');
                expect(value).not.toBe(key);
                expect(collectPlaceholders(value), `${locale} ${key}`).toEqual(
                    collectPlaceholders(String(readPath(en, key)))
                );
            }
        }
    });

    it('does not fall back to English for the primary user actions', () => {
        const primaryKeys = [
            'profile_backup.header',
            'profile_backup.tools_description',
            'profile_backup.unencrypted_warning_title',
            'profile_backup.retry_save',
            'profile_backup.restore_and_restart'
        ];

        for (const locale of languageCodes) {
            if (locale === 'en') {
                continue;
            }
            const source = readLocaleSource(locale);
            for (const key of primaryKeys) {
                expect(readPath(source, key), `${locale} ${key}`).not.toBe(
                    readPath(en, key)
                );
            }
        }
    });

    it('uses natural Japanese product language without internal pipeline terms', () => {
        expect(readPath(ja, 'profile_backup.header')).toBe(
            'バックアップと復元'
        );
        expect(readPath(ja, 'profile_backup.location_not_set')).toBe(
            'バックアップ先が選択されていません'
        );
        expect(readPath(ja, 'profile_backup.retry_save')).toBe('保存を再試行');
        expect(
            readPath(ja, 'profile_backup.unencrypted_warning_title')
        ).toContain('暗号化されていません');
        expect(readPath(ja, 'profile_backup.restore_and_restart')).toBe(
            '復元して再起動'
        );
        expect(readPath(ja, 'profile_backup.phase_snapshot')).toBe(
            'データを準備しています…'
        );
        expect(readPath(ja, 'profile_backup.phase_package')).toBe(
            'バックアップを作成しています…'
        );
        expect(readPath(ja, 'profile_backup.phase_deliver')).toBe(
            'バックアップ先に保存しています…'
        );
        expect(readPath(ja, 'profile_backup.phase_finalize')).toBe(
            '保存を完了しています…'
        );

        const japaneseProfileBackupText = requiredProfileBackupKeys
            .map((key) => readPath(ja, key))
            .filter((value): value is string => typeof value === 'string')
            .join('\n');
        expect(japaneseProfileBackupText).not.toMatch(
            /ステージング|アーティファクト|ロールバック|スナップショット|デリバリー/
        );
    });
});

describe('custom font locale coverage', () => {
    const fontKeyPrefix = 'view.settings.appearance.appearance';
    const fontKeys = (keys: string[]) =>
        keys.map((key) => `${fontKeyPrefix}.${key}`);
    const requiredFontKeys = fontKeys([
        'font_family',
        'cjk_font_pack',
        'font_family_description',
        'font_family_custom',
        'font_family_custom_dialog_title',
        'font_family_custom_dialog_description',
        'font_family_custom_invalid',
        'font_family_custom_primary',
        'font_family_custom_primary_description',
        'font_family_custom_secondary',
        'font_family_custom_secondary_description',
        'font_family_custom_detection_unavailable_title',
        'font_family_custom_detection_unavailable',
        'font_family_custom_detection_unavailable_toast',
        'font_family_custom_mode_label',
        'font_family_custom_mode_installed',
        'font_family_custom_mode_css',
        'font_family_custom_search_placeholder',
        'font_family_custom_search_optional_placeholder',
        'font_family_custom_no_results',
        'font_family_custom_preview',
        'font_family_custom_preview_sample',
        'font_family_custom_override',
        'font_family_custom_override_description',
        'font_family_custom_override_hint',
        'font_family_custom_override_placeholder'
    ]);
    const localizedFontKeys = fontKeys([
        'font_family',
        'font_family_description',
        'font_family_custom',
        'font_family_custom_dialog_title',
        'font_family_custom_dialog_description',
        'font_family_custom_primary',
        'font_family_custom_primary_description',
        'font_family_custom_secondary',
        'font_family_custom_secondary_description',
        'font_family_custom_detection_unavailable_title',
        'font_family_custom_detection_unavailable',
        'font_family_custom_mode_label',
        'font_family_custom_mode_installed',
        'font_family_custom_search_placeholder',
        'font_family_custom_search_optional_placeholder',
        'font_family_custom_no_results',
        'font_family_custom_preview'
    ]);

    it('keeps custom font labels in every locale source file', () => {
        for (const locale of languageCodes) {
            const source = readLocaleSource(locale);
            for (const key of requiredFontKeys) {
                const value = readPath(source, key);
                expect(value, `${locale} ${key}`).toEqual(expect.any(String));
                if (typeof value !== 'string') {
                    continue;
                }
                expect(value.trim()).not.toBe('');
                expect(value).not.toBe(key);
            }
        }
    });

    it('does not fall back to English for user-facing font labels', () => {
        for (const locale of languageCodes) {
            if (locale === 'en') {
                continue;
            }
            const source = readLocaleSource(locale);
            for (const key of localizedFontKeys) {
                expect(readPath(source, key), `${locale} ${key}`).not.toBe(
                    readPath(en, key)
                );
            }
        }
    });
});

describe('deep link locale coverage', () => {
    const requiredDeepLinkKeys = [
        'deep_link.import_collection.confirm.title',
        'deep_link.import_collection.confirm.description',
        'deep_link.import_collection.confirm.worlds_preview',
        'deep_link.import_collection.confirm.confirm',
        'deep_link.import_collection.confirm.cancel',
        'deep_link.import_collection.prompt.title',
        'deep_link.import_collection.prompt.description',
        'deep_link.import_collection.toast.preview_failed',
        'deep_link.import_collection.toast.empty',
        'deep_link.import_collection.toast.import_success',
        'deep_link.import_collection.toast.import_failed',
        'deep_link.import_collection.toast.import_partial_failed',
        'deep_link.import_collection.unknown_author',
        'status_bar.world_collection_importing'
    ];

    it('keeps deep link labels in every locale source file', () => {
        for (const locale of languageCodes) {
            const source = readLocaleSource(locale);
            for (const key of requiredDeepLinkKeys) {
                const value = readPath(source, key);
                expect(value, `${locale} ${key}`).toEqual(expect.any(String));
                if (typeof value !== 'string') {
                    continue;
                }
                expect(value.trim()).not.toBe('');
                expect(value).not.toBe(key);
            }
        }
    });
});

describe('LLM endpoint preset locale coverage', () => {
    const requiredPresetKeys = [
        'view.tools.llm_endpoints.preset',
        'view.tools.llm_endpoints.preset_custom',
        'view.tools.llm_endpoints.presets.openai',
        'view.tools.llm_endpoints.presets.openrouter',
        'view.tools.llm_endpoints.presets.gemini',
        'view.tools.llm_endpoints.presets.deepseek',
        'view.tools.llm_endpoints.presets.xai'
    ];

    it('keeps preset labels in every locale source file', () => {
        for (const locale of languageCodes) {
            const source = readLocaleSource(locale);
            for (const key of requiredPresetKeys) {
                const value = readPath(source, key);
                expect(value, `${locale} ${key}`).toEqual(expect.any(String));
                if (typeof value !== 'string') {
                    continue;
                }
                expect(value.trim()).not.toBe('');
                expect(value).not.toBe(key);
            }
        }
    });
});

function readLocaleSource(locale: string): unknown {
    return localeSources[locale];
}

function readPath(source: unknown, keyPath: string): unknown {
    return keyPath.split('.').reduce<unknown>((value, key) => {
        if (isRecord(value) && key in value) {
            return value[key];
        }
        return undefined;
    }, source);
}

function collectStringPaths(source: unknown, prefix: string): string[] {
    if (!isRecord(source)) {
        return [];
    }
    return Object.entries(source).flatMap(([key, value]) => {
        const path = `${prefix}.${key}`;
        return typeof value === 'string'
            ? [path]
            : collectStringPaths(value, path);
    });
}

function collectPlaceholders(value: string): string[] {
    return Array.from(
        value.matchAll(/\{([^{}]+)\}/g),
        (match) => match[1]
    ).sort();
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null;
}
