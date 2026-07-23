import {
    BookmarkIcon,
    ClockIcon,
    EraserIcon,
    HistoryIcon,
    SquarePenIcon
} from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { isActionRecent } from '@/services/recentActionService';
import { userStatusIndicatorClassName } from '@/shared/utils/userStatus';

import type { SidebarFriendRecord } from './friendsSidebarModel';

const statusOptions = [
    { value: 'join me', labelKey: 'dialog.user.status.join_me' },
    { value: 'active', labelKey: 'dialog.user.status.online' },
    { value: 'ask me', labelKey: 'dialog.user.status.ask_me' },
    { value: 'busy', labelKey: 'dialog.user.status.busy' }
];

export type StatusPreset = {
    status?: unknown;
    statusDescription?: unknown;
};

type ContextMenuItemComponent = ComponentType<{
    children?: ReactNode;
    checked?: boolean;
    disabled?: boolean;
    onClick?: () => void;
}>;

type ContextMenuContainerComponent = ComponentType<{
    children?: ReactNode;
}>;

type ContextMenuSubTriggerComponent = ComponentType<{
    children?: ReactNode;
}>;

type ContextMenuSeparatorComponent = ComponentType;

function statusPresetLabel(
    preset: StatusPreset | null | undefined,
    t: (key: string) => string
) {
    if (preset?.statusDescription) {
        return String(preset.statusDescription);
    }
    const option = statusOptions.find((row) => row.value === preset?.status);
    return option ? t(option.labelKey) : String(preset?.status || '');
}

export function CurrentUserActionItems({
    friend,
    onOpen,
    onChangeStatus,
    onSetStatusDescription,
    onEditStatusDescription,
    onApplyStatusPreset,
    MenuItem,
    CheckboxItem,
    Group,
    Separator,
    Sub,
    SubTrigger,
    SubContent,
    statusPresets = []
}: {
    friend: SidebarFriendRecord & { statusHistory?: unknown };
    onOpen?: () => void;
    onChangeStatus?: (status: string) => void;
    onSetStatusDescription?: (statusDescription: string) => void;
    onEditStatusDescription?: () => void;
    onApplyStatusPreset?: (preset: StatusPreset) => void;
    MenuItem: ContextMenuItemComponent;
    CheckboxItem: ContextMenuItemComponent;
    Group: ContextMenuContainerComponent;
    Separator: ContextMenuSeparatorComponent;
    Sub: ContextMenuContainerComponent;
    SubTrigger: ContextMenuSubTriggerComponent;
    SubContent: ContextMenuContainerComponent;
    statusPresets?: StatusPreset[];
}) {
    const { t } = useTranslation();
    const statusHistory = Array.isArray(friend?.statusHistory)
        ? friend.statusHistory.slice(0, 10)
        : [];

    return (
        <>
            <Group>
                <MenuItem onClick={onOpen}>{t('common.actions.open')}</MenuItem>
            </Group>
            <Separator />
            <Group>
                {statusOptions.map((option) => (
                    <CheckboxItem
                        key={option.value}
                        checked={friend?.status === option.value}
                        onClick={() => {
                            onChangeStatus?.(option.value);
                        }}
                    >
                        <i
                            aria-hidden="true"
                            className={userStatusIndicatorClassName(
                                option.value,
                                { className: 'mr-2' }
                            )}
                        />
                        {t(option.labelKey)}
                    </CheckboxItem>
                ))}
            </Group>
            <Separator />
            <Group>
                <MenuItem
                    onClick={() => {
                        onEditStatusDescription?.();
                    }}
                >
                    <SquarePenIcon className="mr-2 opacity-70" />
                    {t(
                        'view.settings.general.automation.change_status_description'
                    )}
                </MenuItem>
                {friend?.statusDescription ? (
                    <MenuItem
                        onClick={() => {
                            onSetStatusDescription?.('');
                        }}
                    >
                        <EraserIcon className="mr-2 opacity-70" />
                        {t('side_panel.status_menu.clear_description')}
                    </MenuItem>
                ) : null}
                {statusHistory.length ? (
                    <Sub>
                        <SubTrigger>
                            <HistoryIcon className="mr-2 opacity-70" />
                            {t('side_panel.status_menu.recently_used')}
                        </SubTrigger>
                        <SubContent>
                            {statusHistory.map((item, index) => (
                                <CheckboxItem
                                    key={`${item}:${index}`}
                                    checked={friend?.statusDescription === item}
                                    onClick={() => {
                                        onSetStatusDescription?.(String(item));
                                    }}
                                >
                                    <span className="max-w-52 truncate">
                                        {String(item)}
                                    </span>
                                </CheckboxItem>
                            ))}
                        </SubContent>
                    </Sub>
                ) : null}
                {statusPresets.length ? (
                    <Sub>
                        <SubTrigger>
                            <BookmarkIcon className="mr-2 opacity-70" />
                            {t('side_panel.status_menu.presets')}
                        </SubTrigger>
                        <SubContent>
                            {statusPresets.map((preset, index) => (
                                <MenuItem
                                    key={`${preset?.status || 'status'}:${preset?.statusDescription || ''}:${index}`}
                                    onClick={() => {
                                        onApplyStatusPreset?.(preset);
                                    }}
                                >
                                    <i
                                        aria-hidden="true"
                                        className={userStatusIndicatorClassName(
                                            String(preset?.status || 'active'),
                                            { className: 'mr-2' }
                                        )}
                                    />
                                    <span className="max-w-52 truncate">
                                        {statusPresetLabel(preset, t)}
                                    </span>
                                </MenuItem>
                            ))}
                        </SubContent>
                    </Sub>
                ) : null}
            </Group>
        </>
    );
}

export function FriendActionItems({
    friend,
    friendLocation,
    canUseFriendLocation,
    canSendInvite,
    canRequestInvite,
    canBoop,
    onOpen,
    onLaunch,
    onSelfInvite,
    onInvite,
    onRequestInvite,
    onBoop,
    MenuItem,
    Group,
    Separator,
    recentActionVersion = 0
}: {
    friend: SidebarFriendRecord;
    friendLocation?: unknown;
    canUseFriendLocation?: boolean;
    canSendInvite?: boolean;
    canRequestInvite?: boolean;
    canBoop?: boolean;
    onOpen?: () => void;
    onLaunch?: (location: unknown) => void;
    onSelfInvite?: (location: unknown) => void;
    onInvite?: (friend: SidebarFriendRecord) => void;
    onRequestInvite?: (friend: SidebarFriendRecord) => void;
    onBoop?: (friend: SidebarFriendRecord) => void;
    MenuItem: ContextMenuItemComponent;
    Group: ContextMenuContainerComponent;
    Separator: ContextMenuSeparatorComponent;
    recentActionVersion?: number;
}) {
    const { t } = useTranslation();
    const recentInvite =
        recentActionVersion >= 0 && isActionRecent(friend?.id, 'Invite');
    const recentRequestInvite =
        recentActionVersion >= 0 &&
        isActionRecent(friend?.id, 'Request Invite');
    return (
        <>
            <Group>
                <MenuItem onClick={onOpen}>{t('common.actions.open')}</MenuItem>
            </Group>
            <Separator />
            <Group>
                <MenuItem
                    disabled={!canUseFriendLocation}
                    onClick={() => {
                        onLaunch?.(friendLocation);
                    }}
                >
                    {t('dialog.user.info.launch_invite_tooltip')}
                </MenuItem>
                <MenuItem
                    disabled={!canUseFriendLocation}
                    onClick={() => {
                        onSelfInvite?.(friendLocation);
                    }}
                >
                    {t('dialog.user.info.self_invite_tooltip')}
                </MenuItem>
            </Group>
            <Separator />
            <Group>
                <MenuItem
                    disabled={!canSendInvite}
                    onClick={() => {
                        onInvite?.(friend);
                    }}
                >
                    <span className="min-w-0 flex-1">
                        {t('dialog.user.actions.invite')}
                    </span>
                    {recentInvite ? (
                        <ClockIcon className="text-muted-foreground ml-auto" />
                    ) : null}
                </MenuItem>
                <MenuItem
                    disabled={!canRequestInvite}
                    onClick={() => {
                        onRequestInvite?.(friend);
                    }}
                >
                    <span className="min-w-0 flex-1">
                        {t('dialog.user.actions.request_invite')}
                    </span>
                    {recentRequestInvite ? (
                        <ClockIcon className="text-muted-foreground ml-auto" />
                    ) : null}
                </MenuItem>
                <MenuItem
                    disabled={!canBoop}
                    onClick={() => {
                        onBoop?.(friend);
                    }}
                >
                    {t('dialog.user.actions.send_boop')}
                </MenuItem>
            </Group>
        </>
    );
}
