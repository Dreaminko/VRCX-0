import { describe, expect, it } from 'vitest';

import {
    applyResolution,
    getConfigFieldValue,
    getResolutionKey,
    normalizeVrchatConfigForSave,
    parseVrchatConfig
} from './vrchatConfigModel';

describe('vrchatConfigModel', () => {
    it('accepts open VRChat configuration objects while rejecting other JSON roots', () => {
        expect(
            parseVrchatConfig('{"cache_size": 30, "future": {"on": true}}')
        ).toEqual({
            cache_size: 30,
            future: { on: true }
        });
        expect(() => parseVrchatConfig('[]')).toThrow(
            'VRChat configuration must be a JSON object.'
        );
    });

    it('preserves unknown fields and normalizes only values owned by the form', () => {
        expect(
            normalizeVrchatConfigForSave({
                cache_size: '45',
                cache_directory: '',
                disableRichPresence: false,
                picture_output_split_by_date: true,
                future: { on: true }
            })
        ).toEqual({ cache_size: 45, future: { on: true } });
    });

    it('maps configured and default resolutions without changing other keys', () => {
        expect(getResolutionKey({ width: 2560, height: 1440 })).toBe(
            '2560x1440'
        );
        expect(getResolutionKey({ width: '', height: '' })).toBe('__default__');
        expect(
            applyResolution({ future: true }, 'camera_res', '1920x1080')
        ).toEqual({
            future: true,
            camera_res_width: 1920,
            camera_res_height: 1080
        });
    });

    it('exposes only input-compatible scalar field values', () => {
        expect(getConfigFieldValue({ path: 'C:\\VRChat' }, 'path')).toBe(
            'C:\\VRChat'
        );
        expect(getConfigFieldValue({ future: { on: true } }, 'future')).toBe(
            ''
        );
    });
});
