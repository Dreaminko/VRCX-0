// @vitest-environment jsdom

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', async (importOriginal) => ({
    ...(await importOriginal<typeof import('react-i18next')>()),
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock('@/components/instances/InstanceActionBar', () => ({
    InstanceActionBar: () => null
}));

vi.mock('@/components/Location', () => ({
    Location: ({ location }: { location: string }) => <span>{location}</span>
}));

import { InstanceHistoryRow } from './InstanceHistoryList';

describe('InstanceHistoryRow', () => {
    it('keeps row selection and deletion as separate native buttons', async () => {
        const user = userEvent.setup();
        const row = {
            createdAt: '2026-07-01T10:00:00Z',
            location: 'wrld_test:1',
            time: 60_000,
            events: [1]
        };
        const onOpenDetails = vi.fn();
        const onDeleteRow = vi.fn();

        render(
            <InstanceHistoryRow
                row={row}
                selected={false}
                onOpenDetails={onOpenDetails}
                onDeleteRow={onDeleteRow}
            />
        );

        const selectionButton = screen.getByRole('button', {
            name: /wrld_test:1/
        });
        const deleteButton = screen.getByRole('button', {
            name: 'common.actions.delete'
        });

        expect(selectionButton.tagName).toBe('BUTTON');
        await user.click(deleteButton);
        expect(onDeleteRow).toHaveBeenCalledWith(row);
        expect(onOpenDetails).not.toHaveBeenCalled();

        await user.click(selectionButton);
        expect(onOpenDetails).toHaveBeenCalledWith(row);
    });
});
