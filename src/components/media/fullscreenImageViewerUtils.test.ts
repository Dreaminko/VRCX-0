import { afterEach, describe, expect, it, vi } from 'vitest';

import {
    deriveImageFileName,
    toFullSizeImageUrl
} from './fullscreenImageViewerUtils';

describe('fullscreenImageViewerUtils', () => {
    afterEach(() => {
        vi.unstubAllGlobals();
    });

    it('converts a VRChat thumbnail URL to its full-size file URL', () => {
        expect(
            toFullSizeImageUrl(
                'https://api.vrchat.cloud/api/1/image/file_abc-123/7/256?etag=1'
            )
        ).toBe(
            'https://api.vrchat.cloud/api/1/file/file_abc-123/7/file?etag=1'
        );
        expect(toFullSizeImageUrl('https://example.com/image.png')).toBe(
            'https://example.com/image.png'
        );
    });

    it('prefers and sanitizes an explicit file name', () => {
        expect(
            deriveImageFileName({
                fileName: ' folder/bad:name ',
                url: 'https://example.com/ignored.jpg'
            })
        ).toBe('folder_bad_name.png');
        expect(
            deriveImageFileName({
                fileName: 'already.webp',
                url: 'https://example.com/ignored.jpg'
            })
        ).toBe('already.webp');
    });

    it('uses a VRChat file id before URL and source-path fallbacks', () => {
        expect(
            deriveImageFileName({
                url: 'https://api.vrchat.cloud/api/1/file/file_abc-123/7/file',
                sourcePath: 'C:\\screenshots\\fallback.png'
            })
        ).toBe('file_abc-123.png');
    });

    it('decodes an ordinary URL file name and falls back to the source path', () => {
        vi.stubGlobal('window', {
            location: { href: 'https://app.local/' }
        });

        expect(
            deriveImageFileName({
                url: 'https://example.com/photos/photo%20one.jpg?size=large'
            })
        ).toBe('photo one.jpg');
        expect(
            deriveImageFileName({
                url: 'data:image/png;base64,AAAA',
                sourcePath: 'C:\\screenshots\\capture'
            })
        ).toBe('capture.png');
        expect(deriveImageFileName({})).toBe('image.png');
    });
});
