import React, { type ComponentProps } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

vi.mock('./FriendsSidebarFriendRow', () => ({
    FriendRow: ({ rowModel }: { rowModel: { canRequestInvite?: boolean } }) => (
        <button disabled={!rowModel.canRequestInvite}>Request invite</button>
    )
}));

import { FriendsSidebarVirtualRow } from './FriendsSidebarVirtualRows';

type VirtualRowProps = ComponentProps<typeof FriendsSidebarVirtualRow>;

function renderFriendRow({
    isCurrentUser = false,
    state = 'offline'
}: {
    isCurrentUser?: boolean;
    state?: string;
}) {
    const props: VirtualRowProps = {
        appearance: {},
        friendCommands: {
            onOpenFriend: vi.fn(),
            onToggleSection: vi.fn()
        },
        location: { locationMetadataByKey: new Map() },
        row: {
            type: 'friend',
            key: 'friend:test',
            friend: { id: 'usr_friend', state },
            isCurrentUser
        },
        runtime: {
            currentUser: null,
            currentUserId: 'usr_current',
            gameState: { isGameRunning: false },
            onlineIdSet: new Set(),
            instanceActionGatesByUserId: new Map([
                [
                    'usr_friend',
                    {
                        key: 'usr_friend',
                        canJoin: false,
                        canOpenInGame: false,
                        canSelfInvite: false,
                        canRequestInvite: false,
                        canInvite: false
                    }
                ]
            ])
        },
        statusCommands: {}
    };

    return renderToStaticMarkup(<FriendsSidebarVirtualRow {...props} />);
}

describe('FriendsSidebarVirtualRow request invite action', () => {
    it.each(['online', 'offline'])(
        'keeps request invite enabled for a %s friend regardless of instance gates',
        (state) => {
            expect(renderFriendRow({ state })).toBe(
                '<button>Request invite</button>'
            );
        }
    );

    it('keeps request invite unavailable for the current user', () => {
        expect(renderFriendRow({ isCurrentUser: true })).toContain(
            'disabled=""'
        );
    });
});
