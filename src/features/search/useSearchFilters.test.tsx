// @vitest-environment jsdom

import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { MemoryRouter, useLocation } from 'react-router';
import { describe, expect, it } from 'vitest';

import { useSearchFilters } from './useSearchFilters';

function wrapper({ children }: { children: ReactNode }) {
    return (
        <MemoryRouter initialEntries={['/search?tab=world']}>
            {children}
        </MemoryRouter>
    );
}

describe('useSearchFilters', () => {
    it('opens a linked tab and keeps the URL in sync with tab changes', () => {
        const { result } = renderHook(
            () => ({ filters: useSearchFilters(), location: useLocation() }),
            { wrapper }
        );

        expect(result.current.filters.activeTab).toBe('world');
        expect(result.current.location.search).toBe('?tab=world');

        act(() => result.current.filters.setActiveTab('avatar'));
        expect(result.current.filters.activeTab).toBe('avatar');
        expect(result.current.location.search).toBe('?tab=avatar');

        act(() => result.current.filters.setActiveTab('user'));
        expect(result.current.filters.activeTab).toBe('user');
        expect(result.current.location.search).toBe('');
    });
});
