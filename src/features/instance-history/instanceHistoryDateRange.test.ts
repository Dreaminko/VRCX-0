import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';

import {
    buildDefaultInstanceHistoryDateRange,
    buildLocalDayInstanceHistoryDateRange,
    refreshDefaultInstanceHistoryDateRange,
    resolveClearedInstanceHistoryDateRange,
    resolveScopedInstanceHistoryDateRange
} from './instanceHistoryDateRange';

beforeAll(() => {
    vi.stubEnv('TZ', 'America/New_York');
});

afterAll(() => {
    vi.unstubAllEnvs();
});

describe('instanceHistoryDateRange', () => {
    it('builds the default window from the current system time', () => {
        const now = new Date('2026-07-03T12:00:00.000Z');

        expect(buildDefaultInstanceHistoryDateRange(now)).toEqual({
            from: new Date('2026-06-03T12:00:00.000Z'),
            to: now
        });
    });

    it('builds an inclusive local-day window for day mode queries', () => {
        const range = buildLocalDayInstanceHistoryDateRange('2026-07-03');

        expect(range.from).toEqual(new Date(2026, 6, 3, 0, 0, 0, 0));
        expect(range.to).toEqual(new Date(2026, 6, 3, 23, 59, 59, 999));
    });

    it('uses local calendar boundaries across daylight-saving transitions', () => {
        const springForward =
            buildLocalDayInstanceHistoryDateRange('2026-03-08');
        const fallBack = buildLocalDayInstanceHistoryDateRange('2026-11-01');

        expect(springForward.to).toEqual(new Date(2026, 2, 8, 23, 59, 59, 999));
        expect(
            springForward.to!.getTime() - springForward.from!.getTime()
        ).toBe(23 * 60 * 60 * 1000 - 1);
        expect(fallBack.to).toEqual(new Date(2026, 10, 1, 23, 59, 59, 999));
        expect(fallBack.to!.getTime() - fallBack.from!.getTime()).toBe(
            25 * 60 * 60 * 1000 - 1
        );
    });

    it('resets cleared self search dates to the 30 day default', () => {
        const now = new Date('2026-07-03T12:00:00.000Z');

        expect(
            resolveClearedInstanceHistoryDateRange({
                isDayMode: false,
                isSelfScope: true,
                now
            })
        ).toEqual({
            range: {
                from: new Date('2026-06-03T12:00:00.000Z'),
                to: now
            },
            source: 'default'
        });
    });

    it('lets another-user searches explicitly clear to an unbounded value', () => {
        const now = new Date('2026-07-03T12:00:00.000Z');

        expect(
            resolveClearedInstanceHistoryDateRange({
                isDayMode: false,
                isSelfScope: false,
                now
            })
        ).toEqual({
            range: { from: null, to: null },
            source: 'unbounded'
        });
        expect(
            resolveClearedInstanceHistoryDateRange({
                isDayMode: true,
                isSelfScope: true,
                now
            })
        ).toEqual({
            range: { from: null, to: null },
            source: 'none'
        });
    });

    it('adds the bounded default for every search scope with no user date', () => {
        const now = new Date('2026-07-03T12:00:00.000Z');

        expect(
            resolveScopedInstanceHistoryDateRange({
                isDayMode: false,
                isSelfScope: true,
                state: {
                    range: { from: null, to: null },
                    source: 'none'
                },
                now
            })
        ).toEqual({
            range: {
                from: new Date('2026-06-03T12:00:00.000Z'),
                to: now
            },
            source: 'default'
        });
        expect(
            resolveScopedInstanceHistoryDateRange({
                isDayMode: false,
                isSelfScope: false,
                state: {
                    range: { from: null, to: null },
                    source: 'none'
                },
                now
            })
        ).toEqual({
            range: {
                from: new Date('2026-06-03T12:00:00.000Z'),
                to: now
            },
            source: 'default'
        });
        expect(
            resolveScopedInstanceHistoryDateRange({
                isDayMode: true,
                isSelfScope: true,
                state: {
                    range: { from: null, to: null },
                    source: 'none'
                },
                now
            })
        ).toEqual({
            range: { from: null, to: null },
            source: 'none'
        });
    });

    it('preserves defaults and user dates but restores the self bound after an unbounded user search', () => {
        const userRange = {
            from: new Date('2026-01-01T00:00:00.000Z'),
            to: new Date('2026-01-02T00:00:00.000Z')
        };

        expect(
            resolveScopedInstanceHistoryDateRange({
                isDayMode: false,
                isSelfScope: false,
                state: {
                    range: userRange,
                    source: 'default'
                }
            })
        ).toEqual({
            range: userRange,
            source: 'default'
        });
        expect(
            resolveScopedInstanceHistoryDateRange({
                isDayMode: false,
                isSelfScope: false,
                state: {
                    range: userRange,
                    source: 'user'
                }
            })
        ).toEqual({
            range: userRange,
            source: 'user'
        });
        expect(
            resolveScopedInstanceHistoryDateRange({
                isDayMode: false,
                isSelfScope: true,
                state: {
                    range: { from: null, to: null },
                    source: 'unbounded'
                },
                now: new Date('2026-07-03T12:00:00.000Z')
            })
        ).toEqual({
            range: {
                from: new Date('2026-06-03T12:00:00.000Z'),
                to: new Date('2026-07-03T12:00:00.000Z')
            },
            source: 'default'
        });
    });

    it('refreshes only the automatic default window', () => {
        const oldDefaultRange = {
            from: new Date('2026-06-01T12:00:00.000Z'),
            to: new Date('2026-07-01T12:00:00.000Z')
        };
        const userRange = {
            from: new Date('2026-01-01T00:00:00.000Z'),
            to: new Date('2026-01-02T00:00:00.000Z')
        };
        const now = new Date('2026-07-03T12:00:00.000Z');

        expect(
            refreshDefaultInstanceHistoryDateRange(
                {
                    range: oldDefaultRange,
                    source: 'default'
                },
                now
            )
        ).toEqual({
            range: {
                from: new Date('2026-06-03T12:00:00.000Z'),
                to: now
            },
            source: 'default'
        });
        expect(
            refreshDefaultInstanceHistoryDateRange(
                {
                    range: userRange,
                    source: 'user'
                },
                now
            )
        ).toEqual({
            range: userRange,
            source: 'user'
        });
    });
});
