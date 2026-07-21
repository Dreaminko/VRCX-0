import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const commandMocks = vi.hoisted(() => ({
    sessionsQuery: vi.fn()
}));
const configMocks = vi.hoisted(() => ({
    getInt: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({
    commands: {
        appGameLogSessionsQuery: commandMocks.sessionsQuery
    }
}));

vi.mock('./configRepository', () => ({
    default: configMocks
}));

vi.mock('./gameLogPersistenceRepository', () => ({
    default: {}
}));

import { queryLatestSessions } from './gameLogRepository';

describe('gameLogRepository', () => {
    beforeEach(() => {
        vi.stubEnv('TZ', 'America/Los_Angeles');
        commandMocks.sessionsQuery.mockReset();
        commandMocks.sessionsQuery.mockResolvedValue({ sessions: [] });
        configMocks.getInt.mockReset();
        configMocks.getInt.mockResolvedValue(0);
    });

    afterEach(() => {
        vi.unstubAllEnvs();
    });

    it('treats date-only session filters as local calendar days', async () => {
        await queryLatestSessions({
            dateFrom: '2026-07-04',
            dateTo: '2026-07-04'
        });

        expect(commandMocks.sessionsQuery).toHaveBeenCalledWith(
            expect.objectContaining({
                dateFrom: '2026-07-04T07:00:00.000Z',
                dateTo: '2026-07-05T06:59:59.999Z'
            })
        );
    });
});
