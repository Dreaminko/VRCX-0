import { describe, expect, it } from 'vitest';

import { formatDateFilter } from '@/lib/dateTime';

import {
    formatWidgetDate,
    formatWidgetTime,
    getWidgetDayKey
} from './dashboardWidgetUtils';

describe('dashboardWidgetUtils timeline formatting', () => {
    it('groups rows by local calendar day and keeps row timestamps time-only', () => {
        const value = new Date(2026, 7, 12, 11, 37).toISOString();

        expect(getWidgetDayKey(value)).toBe('2026-08-12');
        expect(formatWidgetDate(value)).toBe(formatDateFilter(value, 'date'));
        expect(formatWidgetTime(value)).toBe(formatDateFilter(value, 'time'));
    });
});
