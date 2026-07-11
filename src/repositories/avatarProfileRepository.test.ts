import { describe, expect, it } from 'vitest';

import avatarProfileRepository from './avatarProfileRepository';

describe('AvatarProfileRepository', () => {
    it('normalizes the stable avatar fields while preserving nullable metadata', () => {
        const avatar = avatarProfileRepository.normalize({
            id: 'avtr_redacted',
            name: 'Avatar',
            acknowledgements: null,
            attribution: null,
            authorId: 'usr_redacted',
            authorName: 'Author',
            created_at: '2026-01-01T00:00:00.000Z',
            listingDate: null,
            styles: { primary: 'classic', secondary: 'expressive' },
            unityPackages: [
                {
                    id: 'unp_redacted',
                    platform: 'standalonewindows',
                    variant: 'security'
                }
            ],
            updated_at: '2026-01-02T00:00:00.000Z'
        });

        expect(avatar).toMatchObject({
            id: 'avtr_redacted',
            acknowledgements: null,
            attribution: null,
            listingDate: null,
            styles: { primary: 'classic', secondary: 'expressive' },
            unityPackages: [
                { platform: 'standalonewindows', variant: 'security' }
            ],
            $tags: [],
            $timeSpent: 0,
            $memo: '',
            $isCached: false
        });
    });
});
