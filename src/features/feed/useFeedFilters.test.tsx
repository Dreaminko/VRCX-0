// @vitest-environment jsdom

import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useFeedFilters } from './useFeedFilters';

describe('useFeedFilters', () => {
    it('synchronizes the selected friends when the Feed route scope changes', async () => {
        const { result, rerender } = renderHook(
            ({ routeScopedUserIds }: { routeScopedUserIds: string[] }) =>
                useFeedFilters({ routeScopedUserIds }),
            {
                initialProps: {
                    routeScopedUserIds: ['usr_first']
                }
            }
        );

        expect(result.current.scopedUserIds).toEqual(['usr_first']);

        act(() => {
            rerender({ routeScopedUserIds: ['usr_second'] });
        });

        await waitFor(() => {
            expect(result.current.scopedUserIds).toEqual(['usr_second']);
        });
    });
});
