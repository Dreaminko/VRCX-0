import { describe, expect, it, vi } from 'vitest';

import { getFaviconUrl, replaceVrcPackageUrl } from './urlUtils';

describe('getFaviconUrl', () => {
    it('resolves a small site-identity icon for a profile link, so friend/player lists can show which site a link points to', () => {
        expect(getFaviconUrl('https://github.com/vrcx-team')).toBe(
            'https://icons.duckduckgo.com/ip2/github.com.ico'
        );
    });

    it('keys the icon off the domain only, so links that differ only by path or query still share one icon', () => {
        const deepLink = getFaviconUrl(
            'https://twitter.com/someuser/status/12345?ref=profile'
        );
        const rootLink = getFaviconUrl('https://twitter.com/');

        expect(deepLink).toBe(rootLink);
        expect(deepLink).toBe(
            'https://icons.duckduckgo.com/ip2/twitter.com.ico'
        );
    });

    it('shows no icon instead of crashing the profile view when a user-supplied link is empty or malformed', () => {
        const consoleErrorSpy = vi
            .spyOn(console, 'error')
            .mockImplementation(() => undefined);

        expect(getFaviconUrl('')).toBe('');
        expect(getFaviconUrl('not a url')).toBe('');
        expect(getFaviconUrl(undefined)).toBe('');
        expect(getFaviconUrl(null)).toBe('');

        consoleErrorSpy.mockRestore();
    });
});

describe('replaceVrcPackageUrl', () => {
    it('rewrites the internal VRChat API host to the public vrchat.com host, so package download links work outside the API', () => {
        expect(
            replaceVrcPackageUrl(
                'https://api.vrchat.cloud/api/1/file/file_123/1/file'
            )
        ).toBe('https://vrchat.com/api/1/file/file_123/1/file');
    });

    it('leaves URLs that are already on the public host untouched', () => {
        const publicUrl = 'https://vrchat.com/api/1/file/file_123/1/file';
        expect(replaceVrcPackageUrl(publicUrl)).toBe(publicUrl);
    });

    it('returns an empty string instead of a broken link when no package URL is available yet', () => {
        expect(replaceVrcPackageUrl('')).toBe('');
        expect(replaceVrcPackageUrl(undefined)).toBe('');
        expect(replaceVrcPackageUrl(null)).toBe('');
    });
});
