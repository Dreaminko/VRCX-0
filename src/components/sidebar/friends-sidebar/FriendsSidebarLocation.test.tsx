// @vitest-environment jsdom

import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/state/shellStore', () => ({
    useShellStore: <T,>(
        selector: (state: { timeUnitLabels: { m: string; s: string } }) => T
    ): T => selector({ timeUnitLabels: { m: 'm', s: 's' } })
}));

import { FriendInstanceTimer } from './FriendsSidebarLocation';

describe('FriendInstanceTimer', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.setSystemTime(1_700_000_000_000);
    });

    afterEach(() => {
        cleanup();
        vi.useRealTimers();
    });

    it('shows elapsed time in 30-second buckets', async () => {
        render(<FriendInstanceTimer epoch={1_700_000_000_000} />);

        expect(screen.getByText('<30s')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(29_999));
        expect(screen.getByText('<30s')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(1));
        expect(screen.getByText('1m')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(29_999));
        expect(screen.getByText('1m')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(1));
        expect(screen.getByText('1m 30s')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(30_000));
        expect(screen.getByText('2m')).toBeDefined();
    });
});
