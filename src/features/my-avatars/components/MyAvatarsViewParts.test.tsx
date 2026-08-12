// @vitest-environment jsdom

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock('@/services/dialogService', () => ({
    openAvatarDialog: vi.fn()
}));

import type { MyAvatarActionHandler, MyAvatarRow } from '../myAvatarsTypes';
import { AvatarActionMenuItems } from './MyAvatarGridCard';
import { PlatformBadges } from './MyAvatarsViewParts';

describe('My Avatars view parts', () => {
    it('labels icon-only platform badges', () => {
        render(
            <PlatformBadges
                unityPackages={[
                    { platform: 'standalonewindows', variant: 'standard' },
                    { platform: 'android', variant: 'standard' },
                    { platform: 'ios', variant: 'standard' }
                ]}
            />
        );

        expect(screen.getByLabelText('PC')).toBeTruthy();
        expect(screen.getByLabelText('Android')).toBeTruthy();
        expect(screen.getByLabelText('iOS')).toBeTruthy();
    });

    it('identifies the target avatar without changing menu actions', () => {
        const avatar: MyAvatarRow = {
            id: 'avtr_example',
            name: 'Example Avatar',
            releaseStatus: 'private'
        };
        const onAction = vi.fn<MyAvatarActionHandler>();

        render(
            <AvatarActionMenuItems
                avatar={avatar}
                isActive={false}
                disabled={false}
                Item="button"
                Group="div"
                Label="div"
                Separator="hr"
                onAction={onAction}
            />
        );

        expect(screen.getByText('Example Avatar')).toBeTruthy();

        fireEvent.click(screen.getByText('dialog.avatar.actions.make_public'));

        expect(onAction).toHaveBeenCalledWith('makePublic', avatar);
    });
});
