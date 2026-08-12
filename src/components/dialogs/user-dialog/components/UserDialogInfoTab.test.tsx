// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { UserDialogActivitySummaryPanel } from './UserDialogInfoTab';

vi.mock('react-i18next', async (importOriginal) => ({
    ...(await importOriginal<typeof import('react-i18next')>()),
    useTranslation: () => ({
        i18n: { language: 'en', resolvedLanguage: 'en' },
        t: (key: string) => key
    })
}));

afterEach(cleanup);

describe('UserDialogActivitySummaryPanel', () => {
    it('opens instance history from join count but not time together', () => {
        const onOpenInstanceHistory = vi.fn();

        render(
            <UserDialogActivitySummaryPanel
                friendedAt={undefined}
                isCurrentUser={false}
                isFriend
                lastSeen={undefined}
                onOpenInstanceHistory={onOpenInstanceHistory}
                presenceActivityAt={undefined}
                profile={{ id: 'usr_test' }}
                userTimeSpent={0}
                userJoinCount={0}
            />
        );

        fireEvent.click(
            screen.getByRole('button', {
                name: /dialog\.user\.info\.join_count/
            })
        );
        expect(
            screen.queryByRole('button', {
                name: /dialog\.user\.info\.time_together/
            })
        ).toBeNull();
        expect(screen.getByText('dialog.user.info.time_together')).toBeTruthy();
        expect(
            screen
                .getAllByText(/^dialog\.user\.info\./)
                .map((element) => element.textContent)
        ).toEqual([
            'dialog.user.info.activity_summary',
            'dialog.user.info.last_seen',
            'dialog.user.info.last_activity',
            'dialog.user.info.join_count',
            'dialog.user.info.time_together',
            'dialog.user.info.friended',
            'dialog.user.info.date_joined'
        ]);

        expect(onOpenInstanceHistory).toHaveBeenCalledOnce();
    });

    it('hides the friended date for the current user', () => {
        render(
            <UserDialogActivitySummaryPanel
                friendedAt="2026-08-12T00:00:00Z"
                isCurrentUser
                isFriend={false}
                lastSeen={undefined}
                presenceActivityAt={undefined}
                profile={{ id: 'usr_self' }}
                userTimeSpent={0}
                userJoinCount={0}
            />
        );

        expect(screen.queryByText('dialog.user.info.friended')).toBeNull();
        expect(
            screen
                .getAllByText(/^dialog\.user\.info\./)
                .map((element) => element.textContent)
        ).toEqual([
            'dialog.user.info.activity_summary',
            'dialog.user.info.last_activity',
            'dialog.user.info.play_time',
            'dialog.user.info.date_joined'
        ]);
    });

    it('opens Feed from last activity only for friends', () => {
        const onOpenFeed = vi.fn();
        const { rerender } = render(
            <UserDialogActivitySummaryPanel
                friendedAt={undefined}
                isCurrentUser={false}
                isFriend={false}
                lastSeen={undefined}
                onOpenFeed={onOpenFeed}
                presenceActivityAt={'2026-08-12T00:00:00Z'}
                profile={{ id: 'usr_test' }}
                userTimeSpent={0}
                userJoinCount={0}
            />
        );

        expect(
            screen.queryByRole('button', {
                name: /dialog\.user\.info\.last_activity/
            })
        ).toBeNull();

        rerender(
            <UserDialogActivitySummaryPanel
                friendedAt={undefined}
                isCurrentUser={false}
                isFriend
                lastSeen={undefined}
                onOpenFeed={onOpenFeed}
                presenceActivityAt={'2026-08-12T00:00:00Z'}
                profile={{ id: 'usr_test' }}
                userTimeSpent={0}
                userJoinCount={0}
            />
        );

        fireEvent.click(
            screen.getByRole('button', {
                name: /dialog\.user\.info\.last_activity/
            })
        );
        expect(onOpenFeed).toHaveBeenCalledOnce();
    });
});
