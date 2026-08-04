// @vitest-environment jsdom

import { act, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { WorldProfileRecord } from '@/domain/entities/profileEntities';
import { useWorldFactsStore } from '@/state/worldFactsStore';

import { WorldCard } from './SearchResultParts';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) => key
    })
}));

vi.mock('@/components/media/FadeInImage', () => ({
    FadeInImage: () => null
}));

vi.mock('@/services/dialogService', () => ({
    openAvatarDialog: vi.fn(),
    openGroupDialog: vi.fn(),
    openUserDialog: vi.fn(),
    openWorldDialog: vi.fn()
}));

const world = {
    id: 'wrld_search',
    name: 'Search World',
    description: '',
    authorId: 'usr_author',
    authorName: 'Author',
    capacity: 32,
    createdAt: '',
    favorites: 0,
    heat: 0,
    imageUrl: '',
    isLabs: false,
    occupants: 4,
    platforms: [],
    popularity: 0,
    publicationDate: null,
    recommendedCapacity: 16,
    releaseStatus: 'public',
    tags: [],
    thumbnailImageUrl: '',
    updatedAt: '',
    visits: 0
} satisfies WorldProfileRecord;

describe('SearchResultParts WorldCard', () => {
    beforeEach(() => {
        useWorldFactsStore.getState().resetWorldFacts();
    });

    it('updates occupants when a fresh world fact arrives', () => {
        render(<WorldCard world={world} />);

        expect(screen.getByText('Author (4)')).toBeTruthy();

        act(() => {
            useWorldFactsStore.getState().upsertWorldFacts({
                id: world.id,
                occupants: 12
            });
        });

        expect(screen.getByText('Author (12)')).toBeTruthy();

        act(() => {
            useWorldFactsStore.getState().upsertWorldFacts({
                id: world.id,
                occupants: 0
            });
        });

        expect(screen.getByText('Author')).toBeTruthy();
        expect(screen.queryByText('Author (12)')).toBeNull();
    });
});
