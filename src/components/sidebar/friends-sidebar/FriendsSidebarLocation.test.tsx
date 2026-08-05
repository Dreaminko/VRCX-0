// @vitest-environment jsdom

import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/state/shellStore', () => ({
    useShellStore: <T,>(
        selector: (state: { timeUnitLabels: { m: string; s: string } }) => T
    ): T => selector({ timeUnitLabels: { m: 'm', s: 's' } })
}));

import { FriendInstanceTimer } from './FriendsSidebarLocation';

const NOW_MS = 1_700_000_000_000;

describe('FriendInstanceTimer', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        vi.setSystemTime(NOW_MS);
    });

    afterEach(() => {
        cleanup();
        vi.useRealTimers();
    });

    it('shows the first 30-second bucket, then advances by whole minutes', async () => {
        render(<FriendInstanceTimer epoch={NOW_MS} />);

        expect(screen.getByText('<30s')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(29_999));
        expect(screen.getByText('<30s')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(1));
        expect(screen.getByText('1m')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(29_999));
        expect(screen.getByText('1m')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(1));
        expect(screen.getByText('1m')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(29_999));
        expect(screen.getByText('1m')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(1));
        expect(screen.getByText('2m')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(60_000));
        expect(screen.getByText('3m')).toBeDefined();
    });

    it('continues with whole minutes across the hour boundary', async () => {
        render(<FriendInstanceTimer epoch={NOW_MS - 59 * 60_000 - 29_999} />);

        expect(screen.getByText('59m')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(1));
        expect(screen.getByText('1h')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(59_999));
        expect(screen.getByText('1h')).toBeDefined();
        await act(() => vi.advanceTimersByTimeAsync(1));
        expect(screen.getByText('1h 1m')).toBeDefined();
    });
});
