import {
    Gamepad2Icon,
    HistoryIcon,
    LogInIcon,
    MailIcon,
    RefreshCwIcon,
    UsersRoundIcon,
    XCircleIcon
} from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { ReactElement } from 'react';
import type { LucideIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import {
    buildInstanceActionTarget,
    finiteLocationNumber,
    firstFiniteLocationNumber,
    firstNonNegativeLocationNumber,
    normalizeLocationText
} from '@/components/location/locationModel';
import type { LocationObjectRecord } from '@/components/location/locationModel';
import { instanceLocationKey } from '@/domain/presence/instancePresence';
import { formatDateFilter, timeToText } from '@/lib/dateTime';
import { cn } from '@/lib/utils';
import vrchatInstanceRepository from '@/repositories/vrchatInstanceRepository';
import { tryOpenLaunchLocation } from '@/services/directAccessService';
import { recordLocationHintsFromInstances } from '@/services/domainIngestionService';
import { selfInviteToInstance } from '@/services/launchService';
import { hasGroupIdPrefix } from '@/shared/constants/vrchatIds';
import { useInstanceJoinHistoryStore } from '@/state/instanceJoinHistoryStore';
import { useLaunchStore } from '@/state/launchStore';
import { useModalStore } from '@/state/modalStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { Badge } from '@/ui/shadcn/badge';
import { Button } from '@/ui/shadcn/button';
import { Spinner } from '@/ui/shadcn/spinner';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/ui/shadcn/tooltip';

type GroupPermissionRecord = Record<string, unknown> & {
    myMember?: { permissions?: string[]; roleIds?: string[] };
    roles?: Array<{ id?: string; permissions?: string[] }>;
};
type InstanceActionRecord = Record<string, unknown> & {
    userCount?: unknown;
    occupants?: unknown;
    n_users?: unknown;
    users?: unknown[];
    ref?: Partial<InstanceActionRecord>;
    $disabledContentSettings?: string[];
    group?: GroupPermissionRecord;
    owner?: GroupPermissionRecord;
    capacity?: unknown;
    world?: { capacity?: unknown };
    platforms?: Record<string, unknown>;
    ownerId?: unknown;
    closedAt?: unknown;
    gameServerVersion?: unknown;
    queueEnabled?: unknown;
    queueSize?: unknown;
    ageGate?: unknown;
};

function ActionButton({
    label,
    disabled = false,
    disableTooltip = false,
    loading = false,
    icon: Icon,
    onClick
}: {
    label: string;
    disabled?: boolean;
    disableTooltip?: boolean;
    loading?: boolean;
    icon: LucideIcon;
    onClick?: () => void;
}) {
    const button = (
        <Button
            type="button"
            size="icon-xs"
            variant="outline"
            aria-label={label}
            disabled={disabled || loading}
            onClick={onClick}
        >
            {loading ? (
                <Spinner data-icon="inline-start" />
            ) : (
                <Icon data-icon="inline-start" />
            )}
        </Button>
    );

    if (disableTooltip) {
        return button;
    }

    return (
        <Tooltip>
            <TooltipTrigger render={<span>{button}</span>} />
            <TooltipContent>{label}</TooltipContent>
        </Tooltip>
    );
}

function instanceUserCount(instance: InstanceActionRecord | null) {
    if (!instance) {
        return null;
    }
    return firstNonNegativeLocationNumber(
        instance.userCount,
        instance.occupants,
        instance.n_users,
        Array.isArray(instance.users) ? instance.users.length : null
    );
}

function instanceCapacity(instance: InstanceActionRecord | null) {
    if (!instance) {
        return null;
    }
    return firstFiniteLocationNumber(
        instance.capacity,
        instance.world?.capacity
    );
}

function resolveInstanceSource(
    instance: unknown
): InstanceActionRecord | null {
    if (!instance || typeof instance !== 'object') {
        return null;
    }
    const source = instance as InstanceActionRecord;
    const ref = source.ref;
    if (!ref || typeof ref !== 'object') {
        return source;
    }
    return { ...ref, ...source };
}

function platformCount(
    instance: InstanceActionRecord | null,
    platform: string
) {
    return Number(instance?.platforms?.[platform] ?? 0);
}

function disabledContentSettings(instance: InstanceActionRecord | null) {
    return Array.isArray(instance?.$disabledContentSettings)
        ? instance.$disabledContentSettings.filter(Boolean).join(', ')
        : '';
}

function hasGroupPermission(
    group: GroupPermissionRecord | undefined,
    permission: string
) {
    const direct = Array.isArray(group?.myMember?.permissions)
        ? group.myMember.permissions
        : [];
    if (direct.includes('*') || direct.includes(permission)) {
        return true;
    }
    const roleIds = Array.isArray(group?.myMember?.roleIds)
        ? group.myMember.roleIds
        : [];
    return (Array.isArray(group?.roles) ? group.roles : [])
        .filter((role) => Boolean(role.id && roleIds.includes(role.id)))
        .some(
            (role) =>
                Array.isArray(role.permissions) &&
                (role.permissions.includes('*') ||
                    role.permissions.includes(permission))
        );
}

function canCloseInstance(
    instance: InstanceActionRecord | null,
    currentUserId: string | null
) {
    const ownerId = normalizeLocationText(instance?.ownerId);
    if (!ownerId || !currentUserId) {
        return false;
    }
    if (ownerId === currentUserId) {
        return true;
    }
    if (!hasGroupIdPrefix(ownerId)) {
        return false;
    }
    return (
        hasGroupPermission(instance?.group, 'group-instance-moderate') ||
        hasGroupPermission(instance?.owner, 'group-instance-moderate')
    );
}

function InstanceOpenDuration({ joinedAtMs }: { joinedAtMs: number }) {
    const { t } = useTranslation();

    return (
        <div>
            {t('dialog.instance.label.open_for_at_least', {
                duration: timeToText(Date.now() - joinedAtMs)
            })}
        </div>
    );
}

function InstanceInfoTooltip({
    instance,
    disableTooltip = false,
    joinedAtMs = 0,
    children
}: {
    instance: InstanceActionRecord | null;
    disableTooltip?: boolean;
    joinedAtMs?: number;
    children: ReactElement;
    location?: string;
}) {
    const { t } = useTranslation();

    const disabledContent = disabledContentSettings(instance);
    if (disableTooltip) {
        return children;
    }

    return (
        <Tooltip>
            <TooltipTrigger render={children} />
            <TooltipContent className="max-w-sm text-xs">
                <div className="flex flex-col gap-1.5">
                    {instance?.closedAt ? (
                        <div>
                            {t('dialog.instance.label.closed_at')}{' '}
                            {formatDateFilter(instance.closedAt, 'long')}
                        </div>
                    ) : null}
                    {joinedAtMs ? (
                        <InstanceOpenDuration joinedAtMs={joinedAtMs} />
                    ) : null}
                    <div>
                        <span className="text-platform-pc">PC: </span>
                        {platformCount(instance, 'standalonewindows')}
                        <span className="text-platform-quest ml-2">
                            {t('dialog.instance.label.android')}{' '}
                        </span>
                        {platformCount(instance, 'android')}
                    </div>
                    <div>
                        {t('dialog.instance.label.ios')}{' '}
                        {platformCount(instance, 'ios')}
                    </div>
                    {instance?.gameServerVersion ? (
                        <div>
                            {t('dialog.instance.label.game_version')}{' '}
                            {String(instance.gameServerVersion)}
                        </div>
                    ) : null}
                    {instance?.queueEnabled ? (
                        <div>
                            {t(
                                'dialog.instance.label.instance_queuing_enabled'
                            )}
                        </div>
                    ) : null}
                    {disabledContent ? (
                        <div>
                            {t('dialog.instance.label.disabled_content')}{' '}
                            {disabledContent}
                        </div>
                    ) : null}
                </div>
            </TooltipContent>
        </Tooltip>
    );
}

export function InstanceActionBar({
    className,
    target = null,
    location = '',
    launchLocation = '',
    inviteLocation = '',
    instanceLocation = '',
    shortName = '',
    worldName = '',
    instance = null,
    friendCount,
    playerCount,
    capacity: providedCapacity,
    showLaunch = true,
    showInvite = true,
    showRefresh = true,
    showHistory = false,
    showInstanceInfo = true,
    instanceInfoPlacement = 'end',
    instanceCountAlign = 'right',
    instanceSummaryOrder = 'count-first',
    disableTooltip = false,
    disableInstanceInfoTooltip = disableTooltip,
    refreshTooltip = 'Refresh instance info',
    historyTooltip = 'Previous instance history',
    onRefresh,
    onHistory
}: {
    className?: string;
    target?: LocationObjectRecord | null;
    location?: unknown;
    launchLocation?: unknown;
    inviteLocation?: unknown;
    instanceLocation?: unknown;
    shortName?: unknown;
    worldName?: unknown;
    instance?: unknown;
    friendCount?: number;
    playerCount?: unknown;
    capacity?: unknown;
    showLaunch?: boolean;
    showInvite?: boolean;
    showRefresh?: boolean;
    showHistory?: boolean;
    showInstanceInfo?: boolean;
    instanceInfoPlacement?: 'start' | 'end';
    instanceCountAlign?: 'left' | 'right';
    instanceSummaryOrder?: 'count-first' | 'markers-first';
    disableTooltip?: boolean;
    disableInstanceInfoTooltip?: boolean;
    refreshTooltip?: string;
    historyTooltip?: string;
    onRefresh?: (location: string) => unknown | Promise<unknown>;
    onHistory?: () => void;
}) {
    const { t } = useTranslation();

    const endpoint = useRuntimeStore((state) => state.auth.currentUserEndpoint);
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const isGameRunning = useRuntimeStore((state) =>
        Boolean(state.gameState.isGameRunning)
    );
    const confirm = useModalStore((state) => state.confirm);
    const showLaunchDialog = useLaunchStore((state) => state.showLaunchDialog);
    const [busy, setBusy] = useState('');
    const [instanceInfo, setInstanceInfo] = useState(() =>
        resolveInstanceSource(instance)
    );
    const actionTarget = useMemo(
        () =>
            buildInstanceActionTarget({
                target,
                location,
                launchLocation,
                inviteLocation,
                instanceLocation,
                shortName,
                worldName
            }),
        [
            target,
            location,
            launchLocation,
            inviteLocation,
            instanceLocation,
            shortName,
            worldName
        ]
    );
    const joinHistoryKey = useMemo(
        () => instanceLocationKey(actionTarget.instanceLocation),
        [actionTarget.instanceLocation]
    );
    const joinedAtMs = useInstanceJoinHistoryStore(
        (state) =>
            (joinHistoryKey ? state.joinedAtByLocation[joinHistoryKey] : 0) || 0
    );
    const userCount = instanceUserCount(instanceInfo);
    const providedPlayerCount = firstNonNegativeLocationNumber(playerCount);
    const resolvedUserCount = userCount ?? providedPlayerCount;
    const capacity =
        instanceCapacity(instanceInfo) ??
        finiteLocationNumber(providedCapacity) ??
        0;
    const hasUserCount = userCount !== null || providedPlayerCount !== null;
    const canCloseCurrentInstance = canCloseInstance(
        instanceInfo,
        currentUserId
    );
    const activeContextRef = useRef<{
        endpoint: string;
        location: string;
    }>({
        endpoint,
        location: actionTarget.instanceLocation
    });
    const hasInstanceSummary = Boolean(
        instanceInfo || hasUserCount || capacity || friendCount || joinedAtMs
    );
    const queueSize = Number(instanceInfo?.queueSize) || 0;
    const hasAgeGate = Boolean(
        instanceInfo?.ageGate ||
        actionTarget.instanceLocation.includes('~ageGate')
    );
    const canShowLaunchAction = showLaunch && actionTarget.isRealLaunchLocation;
    const canOpenInstanceInGame = canShowLaunchAction && isGameRunning;

    useEffect(() => {
        activeContextRef.current = {
            endpoint,
            location: actionTarget.instanceLocation
        };
        setInstanceInfo(resolveInstanceSource(instance));
    }, [endpoint, instance, actionTarget.instanceLocation]);

    function launchInstance() {
        if (!actionTarget.launchLocation || busy) {
            return;
        }
        showLaunchDialog(
            actionTarget.launchLocation,
            actionTarget.parsedLaunchLocation.shortName || '',
            actionTarget.shortName,
            {
                worldName: actionTarget.worldName
            }
        );
    }

    async function openInstanceInGame() {
        if (!canOpenInstanceInGame || busy) {
            return;
        }
        setBusy('open-in-game');
        try {
            const opened = await tryOpenLaunchLocation(
                actionTarget.launchLocation,
                actionTarget.parsedLaunchLocation.shortName ||
                    actionTarget.shortName
            );
            if (opened) {
                toast.success(
                    t('dialog.instance.success.vrchat_launch_request_sent')
                );
                return;
            }
            toast.error(
                t(
                    'dialog.instance.error.unable_to_open_this_instance_in_vrchat'
                )
            );
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.instance_action_bar.toast.failed_to_launch_instance'
                      )
            );
        } finally {
            setBusy('');
        }
    }

    async function selfInvite() {
        if (!actionTarget.isRealInviteLocation || busy) {
            return;
        }
        setBusy('invite');
        try {
            await selfInviteToInstance(
                actionTarget.inviteLocation,
                actionTarget.parsedInviteLocation.shortName ||
                    actionTarget.shortName
            );
            toast.success(t('message.invite.self_sent'));
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.instance_action_bar.toast.failed_to_send_self_invite'
                      )
            );
        } finally {
            setBusy('');
        }
    }

    async function refreshInstance() {
        if (!actionTarget.isRealInstanceLocation || busy) {
            return;
        }
        const requestLocation = actionTarget.instanceLocation;
        const requestEndpoint = endpoint;
        setBusy('refresh');
        try {
            const override = await onRefresh?.(requestLocation);
            if (
                activeContextRef.current.location !== requestLocation ||
                activeContextRef.current.endpoint !== requestEndpoint
            ) {
                return;
            }
            if (override) {
                const normalizedOverride = resolveInstanceSource(override);
                setInstanceInfo(normalizedOverride);
                recordLocationHintsFromInstances({
                    endpoint: requestEndpoint,
                    instances: [normalizedOverride]
                });
            } else {
                const response = await vrchatInstanceRepository.getInstance({
                    worldId: actionTarget.parsedInstanceLocation.worldId,
                    instanceId: actionTarget.parsedInstanceLocation.instanceId
                });
                if (
                    activeContextRef.current.location !== requestLocation ||
                    activeContextRef.current.endpoint !== requestEndpoint
                ) {
                    return;
                }
                setInstanceInfo(resolveInstanceSource(response.json));
                recordLocationHintsFromInstances({
                    endpoint: requestEndpoint,
                    instances: [response.json]
                });
            }
            toast.success(t('dialog.instance.success.instance_refreshed'));
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.instance_action_bar.toast.failed_to_refresh_instance'
                      )
            );
        } finally {
            setBusy('');
        }
    }

    async function closeInstance() {
        if (!actionTarget.instanceLocation || busy) {
            return;
        }
        const requestLocation = actionTarget.instanceLocation;
        const requestEndpoint = endpoint;
        const result = await confirm({
            title: t('confirm.title'),
            description: t('confirm.close_instance'),
            destructive: true
        });
        if (!result.ok) {
            return;
        }

        setBusy('close');
        try {
            const response = await vrchatInstanceRepository.closeInstance({
                location: requestLocation,
                hardClose: false
            });
            if (
                activeContextRef.current.location !== requestLocation ||
                activeContextRef.current.endpoint !== requestEndpoint
            ) {
                return;
            }
            if (response.json) {
                setInstanceInfo(resolveInstanceSource(response.json));
                recordLocationHintsFromInstances({
                    endpoint: requestEndpoint,
                    instances: [response.json]
                });
            }
            toast.success(t('dialog.instance.label.instance_closed'));
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.instance_action_bar.toast.failed_to_close_instance'
                      )
            );
        } finally {
            setBusy('');
        }
    }

    if (
        !actionTarget.instanceLocation &&
        !actionTarget.launchLocation &&
        !actionTarget.inviteLocation
    ) {
        return null;
    }

    const countSummary =
        hasUserCount || capacity ? (
            <span
                className={cn(
                    'inline-block min-w-[5ch] tabular-nums',
                    instanceCountAlign === 'left' ? 'text-left' : 'text-right'
                )}
            >
                {hasUserCount ? resolvedUserCount : '—'}
                {capacity ? `/${capacity}` : ''}
            </span>
        ) : null;

    const markerSummary = (
        <>
            {friendCount ? (
                <span className="inline-flex items-center gap-0.5">
                    <UsersRoundIcon className="size-3.5" />
                    {friendCount}
                </span>
            ) : null}
            {queueSize ? (
                <span>
                    {t('dialog.new_instance.queueEnabled')} {queueSize}
                </span>
            ) : null}
            {hasAgeGate ? (
                <Badge className="bg-amber-500/15 text-amber-300">
                    {t('dialog.new_instance.ageGate')}
                </Badge>
            ) : null}
        </>
    );

    const closeInstanceLabel = t('dialog.instance.action.close_instance');
    const closeInstanceControl =
        showInstanceInfo && canCloseCurrentInstance ? (
            <Button
                type="button"
                size="icon-xs"
                variant="ghost"
                aria-label={closeInstanceLabel}
                disabled={Boolean(busy) || Boolean(instanceInfo?.closedAt)}
                onClick={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    closeInstance();
                }}
            >
                {busy === 'close' ? (
                    <Spinner data-icon="inline-start" />
                ) : (
                    <XCircleIcon data-icon="inline-start" />
                )}
            </Button>
        ) : null;
    const closeInstanceButton =
        closeInstanceControl && !disableTooltip ? (
            <Tooltip>
                <TooltipTrigger render={<span>{closeInstanceControl}</span>} />
                <TooltipContent>{closeInstanceLabel}</TooltipContent>
            </Tooltip>
        ) : (
            closeInstanceControl
        );
    const instanceInfoSummary =
        showInstanceInfo && hasInstanceSummary ? (
            <InstanceInfoTooltip
                instance={instanceInfo}
                location={actionTarget.instanceLocation}
                disableTooltip={disableInstanceInfoTooltip}
                joinedAtMs={joinedAtMs}
            >
                <div className="text-muted-foreground inline-flex items-center gap-1 text-xs">
                    {instanceSummaryOrder === 'markers-first'
                        ? markerSummary
                        : countSummary}
                    {instanceSummaryOrder === 'markers-first'
                        ? countSummary
                        : markerSummary}
                </div>
            </InstanceInfoTooltip>
        ) : null;
    const instanceSummary =
        instanceInfoSummary || closeInstanceButton ? (
            <div className="inline-flex items-center gap-1">
                {instanceSummaryOrder === 'markers-first'
                    ? closeInstanceButton
                    : instanceInfoSummary}
                {instanceSummaryOrder === 'markers-first'
                    ? instanceInfoSummary
                    : closeInstanceButton}
            </div>
        ) : null;

    return (
        <div
            className={cn(
                'inline-flex items-center gap-1.5 align-middle',
                className
            )}
        >
            {instanceInfoPlacement === 'start' ? instanceSummary : null}
            {canShowLaunchAction ? (
                <ActionButton
                    label={t('dialog.instance.action.launch_instance')}
                    icon={LogInIcon}
                    disableTooltip={disableTooltip}
                    loading={busy === 'launch'}
                    disabled={Boolean(busy)}
                    onClick={launchInstance}
                />
            ) : null}
            {canOpenInstanceInGame ? (
                <ActionButton
                    label={t('dialog.instance.action.open_in_game')}
                    icon={Gamepad2Icon}
                    disableTooltip={disableTooltip}
                    loading={busy === 'open-in-game'}
                    disabled={Boolean(busy)}
                    onClick={() => {
                        openInstanceInGame();
                    }}
                />
            ) : null}
            {showInvite && actionTarget.isRealInviteLocation ? (
                <ActionButton
                    label={t('dialog.instance.label.self_invite')}
                    icon={MailIcon}
                    disableTooltip={disableTooltip}
                    loading={busy === 'invite'}
                    disabled={Boolean(busy)}
                    onClick={() => {
                        selfInvite();
                    }}
                />
            ) : null}
            {showRefresh && actionTarget.isRealInstanceLocation ? (
                <ActionButton
                    label={refreshTooltip}
                    icon={RefreshCwIcon}
                    disableTooltip={disableTooltip}
                    loading={busy === 'refresh'}
                    disabled={Boolean(busy)}
                    onClick={() => {
                        refreshInstance();
                    }}
                />
            ) : null}
            {showHistory ? (
                <ActionButton
                    label={historyTooltip}
                    icon={HistoryIcon}
                    disableTooltip={disableTooltip}
                    disabled={Boolean(busy)}
                    onClick={onHistory}
                />
            ) : null}
            {instanceInfoPlacement === 'start' ? null : instanceSummary}
        </div>
    );
}
