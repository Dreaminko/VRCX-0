import { beforeEach, describe, expect, it } from 'vitest';

import { useInstanceJoinHistoryStore } from './instanceJoinHistoryStore';

const LOCATION = 'wrld_test:12345';
const KEY = 'wrld_test:12345';

describe('instanceJoinHistoryStore', () => {
    beforeEach(() => {
        useInstanceJoinHistoryStore.getState().resetInstanceJoinHistory();
    });

    it('keeps the earliest join for a location and drops unusable rows', () => {
        useInstanceJoinHistoryStore.getState().setInstanceJoinHistory([
            [LOCATION, '2026-07-20T12:00:00.000Z'],
            [LOCATION, '2026-07-20T09:00:00.000Z'],
            ['traveling', '2026-07-20T08:00:00.000Z'],
            ['wrld_other:6789', 'not a date'],
            ['', '2026-07-20T07:00:00.000Z']
        ]);

        expect(
            useInstanceJoinHistoryStore.getState().joinedAtByLocation
        ).toEqual({
            [KEY]: Date.parse('2026-07-20T09:00:00.000Z')
        });
    });

    it('normalizes locations that carry a short name to the same key', () => {
        useInstanceJoinHistoryStore
            .getState()
            .setInstanceJoinHistory([
                [`${LOCATION}&shortName=abcdef`, '2026-07-20T12:00:00.000Z']
            ]);

        expect(
            useInstanceJoinHistoryStore.getState().joinedAtByLocation[KEY]
        ).toBe(Date.parse('2026-07-20T12:00:00.000Z'));
    });

    it('records a live join only when it predates the stored value', () => {
        const store = useInstanceJoinHistoryStore.getState();
        store.recordInstanceJoin(LOCATION, '2026-07-20T12:00:00.000Z');
        store.recordInstanceJoin(LOCATION, '2026-07-20T13:00:00.000Z');

        expect(
            useInstanceJoinHistoryStore.getState().joinedAtByLocation[KEY]
        ).toBe(Date.parse('2026-07-20T12:00:00.000Z'));

        store.recordInstanceJoin(LOCATION, '2026-07-20T10:00:00.000Z');

        expect(
            useInstanceJoinHistoryStore.getState().joinedAtByLocation[KEY]
        ).toBe(Date.parse('2026-07-20T10:00:00.000Z'));
    });

    it('ignores joins that carry no resolvable instance or timestamp', () => {
        const store = useInstanceJoinHistoryStore.getState();
        store.recordInstanceJoin('traveling', '2026-07-20T12:00:00.000Z');
        store.recordInstanceJoin(LOCATION, '');
        store.recordInstanceJoin(LOCATION, 0);

        expect(
            useInstanceJoinHistoryStore.getState().joinedAtByLocation
        ).toEqual({});
    });

    it('clears every entry on reset', () => {
        const store = useInstanceJoinHistoryStore.getState();
        store.recordInstanceJoin(LOCATION, '2026-07-20T12:00:00.000Z');

        store.resetInstanceJoinHistory();

        expect(
            useInstanceJoinHistoryStore.getState().joinedAtByLocation
        ).toEqual({});
    });
});
