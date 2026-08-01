// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    openRow: vi.fn()
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) =>
            key === 'dialog.user.mutual_friends.undisclosed_friend'
                ? 'Localized Undisclosed Mutual Friend'
                : key
    })
}));

vi.mock('@/components/user-hover-card/UserHoverCard', () => ({
    UserHoverCard: ({
        children,
        disabled
    }: {
        children: ReactNode;
        disabled?: boolean;
    }) => <div data-hover-disabled={String(Boolean(disabled))}>{children}</div>
}));

vi.mock('@/components/UserStatusAvatar', () => ({
    UserStatusAvatar: () => <span />
}));

vi.mock('@/components/sidebar/friends-sidebar/friendsSidebarModel', () => ({
    resolveSidebarStatusDotClassName: () => ''
}));

vi.mock('@/services/entityMediaService', () => ({
    convertFileUrlToImageUrl: () => '',
    userImage: () => ''
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: <T,>(
        selector: (state: {
            auth: {
                currentUserEndpoint: string;
                currentUserSnapshot: null;
            };
            gameState: { isGameRunning: boolean };
        }) => T
    ): T =>
        selector({
            auth: {
                currentUserEndpoint: 'https://api.vrchat.cloud',
                currentUserSnapshot: null
            },
            gameState: { isGameRunning: false }
        })
}));

vi.mock('./userDialogEntityNavigation', () => ({
    openRow: mocks.openRow
}));

import { EntityList } from './UserDialogEntityList';

describe('UserDialog EntityList', () => {
    afterEach(() => {
        cleanup();
        vi.clearAllMocks();
    });

    it('localizes undisclosed mutual friends and prevents opening them', () => {
        render(
            <EntityList
                kind="user"
                rows={[
                    {
                        id: 'usr_00000000-0000-0000-0000-000000000000',
                        displayName: 'Hidden Mutual'
                    },
                    {
                        id: 'usr_visible',
                        displayName: 'Visible Friend'
                    }
                ]}
            />
        );

        const undisclosedButton = screen.getByRole('button', {
            name: 'Localized Undisclosed Mutual Friend'
        });
        const visibleButton = screen.getByRole('button', {
            name: 'Visible Friend'
        });

        expect(undisclosedButton).toHaveProperty('disabled', true);
        expect(
            undisclosedButton.parentElement?.getAttribute('data-hover-disabled')
        ).toBe('true');
        fireEvent.click(undisclosedButton);
        expect(mocks.openRow).not.toHaveBeenCalled();

        fireEvent.click(visibleButton);
        expect(mocks.openRow).toHaveBeenCalledTimes(1);
    });
});
