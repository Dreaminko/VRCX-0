import type { TFunction } from 'i18next';
import { ReplyIcon } from 'lucide-react';
import { describe, expect, it, vi } from 'vitest';

import type { NotificationRow } from '@/repositories/notificationPersistenceRepository';

import {
    buildOrderedActions,
    type NotificationRowActionHandlers
} from './notificationRowActions';

const t = ((key: string) => key) as TFunction;

function createHandlers(): NotificationRowActionHandlers {
    return {
        onAcceptFriendRequest: vi.fn(),
        onAcceptRequestInvite: vi.fn(),
        onHideNotification: vi.fn(),
        onSendInviteResponseWithMessage: vi.fn(),
        onSendNotificationResponse: vi.fn()
    };
}

function buildActions(
    notification: NotificationRow,
    handlers: NotificationRowActionHandlers
) {
    return buildOrderedActions({
        notification,
        currentUserId: 'usr_self',
        canInviteFromCurrentLocation: false,
        handlers,
        t
    });
}

describe('buildOrderedActions', () => {
    it('adds a manual reply response for a received boop', () => {
        const handlers = createHandlers();
        const notification: NotificationRow = {
            id: 'notif_boop',
            type: 'boop',
            senderUserId: 'usr_sender',
            responses: []
        };

        const actions = buildActions(notification, handlers);

        expect(actions.map((action) => action.key)).toEqual(['reply-boop']);
        expect(actions[0]?.Icon).toBe(ReplyIcon);
        expect(actions[0]?.label).toBe('view.notification.action.send_boop');
        actions[0]?.onClick();
        expect(handlers.onSendNotificationResponse).toHaveBeenCalledWith(
            notification,
            {
                icon: 'reply',
                type: 'reply'
            }
        );
    });

    it('does not expose a boop action without a sender user id', () => {
        const actions = buildActions(
            {
                id: 'notif_boop',
                type: 'boop',
                responses: []
            },
            createHandlers()
        );

        expect(actions).toEqual([]);
    });

    it('keeps the manual reply available after a boop expires', () => {
        const actions = buildActions(
            {
                id: 'notif_boop',
                type: 'boop',
                senderUserId: 'usr_sender',
                expired: true,
                responses: []
            },
            createHandlers()
        );

        expect(actions.map((action) => action.key)).toEqual(['reply-boop']);
    });

    it('keeps actions hidden for other expired notifications', () => {
        const actions = buildActions(
            {
                id: 'notif_invite',
                type: 'invite',
                senderUserId: 'usr_sender',
                expired: true,
                responses: [{ type: 'accept', text: 'Accept' }]
            },
            createHandlers()
        );

        expect(actions).toEqual([]);
    });
});
