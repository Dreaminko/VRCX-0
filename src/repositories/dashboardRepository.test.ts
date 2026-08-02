import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getString:
        vi.fn<(key: string, fallback: string | null) => Promise<unknown>>(),
    setString: vi.fn<(key: string, value: string) => Promise<void>>()
}));

vi.mock('./configRepository', () => ({
    default: {
        getString: mocks.getString,
        setString: mocks.setString
    }
}));

import { DASHBOARD_STORAGE_KEY } from '@/shared/constants/dashboard';

import dashboardRepository from './dashboardRepository';

describe('dashboardRepository persistence', () => {
    beforeEach(() => {
        mocks.getString.mockReset();
        mocks.setString.mockReset().mockResolvedValue(undefined);
    });

    it('returns an empty list for missing, malformed, and invalid stored data', async () => {
        mocks.getString
            .mockResolvedValueOnce(null)
            .mockResolvedValueOnce('{broken')
            .mockResolvedValueOnce(JSON.stringify({ dashboards: [null, {}] }));

        await expect(dashboardRepository.getDashboards()).resolves.toEqual([]);
        await expect(dashboardRepository.getDashboards()).resolves.toEqual([]);
        await expect(dashboardRepository.getDashboards()).resolves.toEqual([]);
    });

    it('normalizes legacy stored dashboards without inventing row ids on load', async () => {
        mocks.getString.mockResolvedValue(
            JSON.stringify({
                dashboards: [
                    {
                        id: ' dashboard-main ',
                        name: '  ',
                        icon: 'LayoutDashboardIcon',
                        rows: [
                            {
                                direction: 'vertical',
                                panels: [
                                    'feed',
                                    { key: 'game-log', config: { days: 7 } },
                                    'ignored-third-panel'
                                ]
                            },
                            { panels: [] }
                        ]
                    }
                ]
            })
        );

        await expect(dashboardRepository.getDashboards()).resolves.toEqual([
            {
                id: 'dashboard-main',
                name: 'Dashboard',
                icon: 'lucide:LayoutDashboard',
                rows: [
                    {
                        direction: 'vertical',
                        panels: [
                            'feed',
                            { key: 'game-log', config: { days: 7 } }
                        ]
                    }
                ]
            }
        ]);
    });

    it('saves sanitized dashboards and returns the exact persisted shape', async () => {
        const saved = await dashboardRepository.saveDashboards([
            {
                id: ' dashboard-main ',
                name: ' Main ',
                icon: 'invalid',
                rows: [
                    {
                        direction: 'sideways',
                        panels: [
                            { key: 'feed', config: { nested: { value: 1 } } },
                            { key: '', config: {} }
                        ]
                    }
                ]
            },
            { id: '', rows: [] }
        ]);

        expect(saved).toHaveLength(1);
        expect(saved[0]).toMatchObject({
            id: 'dashboard-main',
            name: 'Main',
            icon: 'lucide:LayoutDashboard',
            rows: [
                {
                    direction: 'horizontal',
                    panels: [
                        { key: 'feed', config: { nested: { value: 1 } } },
                        null
                    ]
                }
            ]
        });
        expect(saved[0].rows[0].id).toEqual(expect.any(String));
        expect(mocks.setString).toHaveBeenCalledWith(
            DASHBOARD_STORAGE_KEY,
            JSON.stringify({ dashboards: saved })
        );
    });

    it('generates the first available dashboard name', () => {
        expect(
            dashboardRepository.generateNextDashboardName(
                [
                    { name: 'Dashboard' },
                    { name: 'Dashboard 1' },
                    { name: 'Dashboard 3' }
                ],
                ' Dashboard '
            )
        ).toBe('Dashboard 2');
    });
});
