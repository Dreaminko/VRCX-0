import { describe, expect, it } from 'vitest';

import { queryClient } from './queryClient';

describe('queryClient defaults', () => {
    it('retries a failed VRChat API call exactly once instead of hammering the API or failing on the first blip', () => {
        expect(queryClient.getDefaultOptions().queries?.retry).toBe(1);
    });

    it('does not refetch every entity query just because the user tabbed back into the app, to stay under VRChat API rate limits', () => {
        expect(
            queryClient.getDefaultOptions().queries?.refetchOnWindowFocus
        ).toBe(false);
    });

    it('refetches after the network reconnects, so data recovers automatically after sleep or a dropped connection', () => {
        expect(
            queryClient.getDefaultOptions().queries?.refetchOnReconnect
        ).toBe(true);
    });
});
