import { CopyIcon, KeyRoundIcon, RefreshCwIcon } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { commands, type LocalApiStatus } from '@/platform/tauri/bindings';
import { PlatformCommandError } from '@/platform/tauri/errors';
import { copyTextToClipboard } from '@/services/clipboardService';
import { subscribeLocalApiStatusRefresh } from '@/services/localApiService';
import { Button } from '@/ui/shadcn/button';
import { Input } from '@/ui/shadcn/input';
import { Switch } from '@/ui/shadcn/switch';

import { Field, SettingsGroup } from '../SettingsField';

export function LocalApiSettingsGroup() {
    const { t } = useTranslation();
    const [status, setStatus] = useState<LocalApiStatus | null>(null);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [portInput, setPortInput] = useState('8799');

    const applyStatus = useCallback((next: LocalApiStatus) => {
        setStatus(next);
        setPortInput(String(next.port));
        setError(next.lastError?.message ?? null);
    }, []);

    const refreshStatus = useCallback(async () => {
        applyStatus(await commands.appLocalApiStatus());
    }, [applyStatus]);

    useEffect(() => {
        let active = true;
        commands
            .appLocalApiStatus()
            .then((next) => {
                if (active) {
                    applyStatus(next);
                }
            })
            .catch((caught: unknown) => {
                if (active) {
                    setError(errorMessage(caught));
                }
            });
        return () => {
            active = false;
        };
    }, [applyStatus]);

    useEffect(
        () =>
            subscribeLocalApiStatusRefresh(() => {
                void refreshStatus().catch((caught: unknown) => {
                    setError(errorMessage(caught));
                });
            }),
        [refreshStatus]
    );

    async function runCommand(
        action: () => Promise<LocalApiStatus>
    ): Promise<boolean> {
        setBusy(true);
        try {
            applyStatus(await action());
            return true;
        } catch (caught: unknown) {
            const message = localizedError(caught);
            try {
                await refreshStatus();
            } catch {}
            setError(message);
            toast.error(message);
            return false;
        } finally {
            setBusy(false);
        }
    }

    function localizedError(caught: unknown): string {
        if (caught instanceof PlatformCommandError) {
            if (caught.code === 'local_api_port_in_use' && caught.port) {
                return t('view.settings.integrations.local_api.port_in_use', {
                    port: caught.port
                });
            }
            if (caught.code === 'local_api_bind' && caught.port) {
                return t('view.settings.integrations.local_api.bind_failed', {
                    port: caught.port
                });
            }
        }
        return errorMessage(caught);
    }

    function applyPort() {
        const port = Number(portInput);
        if (!Number.isInteger(port) || port < 1024 || port > 65535) {
            toast.error(t('view.settings.integrations.local_api.port_invalid'));
            return;
        }
        void runCommand(() => commands.appLocalApiSetPort(port)).then(
            (succeeded) => {
                if (!succeeded) {
                    setPortInput(String(port));
                }
            }
        );
    }

    async function copyToken() {
        if (!status?.token) {
            return;
        }
        await copyTextToClipboard(status.token, {
            successMessage: t(
                'view.settings.integrations.local_api.token_copied'
            ),
            errorMessage: errorMessage
        });
    }

    return (
        <SettingsGroup
            title={t('view.settings.integrations.local_api.header')}
            description={t('view.settings.integrations.local_api.description')}
        >
            <Field
                label={t('view.settings.integrations.local_api.enable')}
                description={t(
                    'view.settings.integrations.local_api.enable_description'
                )}
            >
                <Switch
                    checked={status?.enabled === true}
                    disabled={busy}
                    onCheckedChange={(enabled) => {
                        void runCommand(() =>
                            commands.appLocalApiSetEnabled(enabled)
                        );
                    }}
                />
            </Field>

            <Field
                label={t(
                    'view.settings.integrations.local_api.allow_lan_connections'
                )}
                description={t(
                    'view.settings.integrations.local_api.allow_lan_connections_description'
                )}
            >
                <Switch
                    checked={status?.allowLanConnections === true}
                    disabled={busy}
                    onCheckedChange={(enabled) => {
                        void runCommand(() =>
                            commands.appLocalApiSetAllowLanConnections(enabled)
                        );
                    }}
                />
            </Field>

            <Field
                label={t('view.settings.integrations.local_api.port_label')}
                description={t(
                    'view.settings.integrations.local_api.port_description'
                )}
                error={error ?? undefined}
            >
                <div className="flex items-center gap-2">
                    <Input
                        type="number"
                        min={1024}
                        max={65535}
                        value={portInput}
                        disabled={busy}
                        onChange={(event) => setPortInput(event.target.value)}
                        className="w-28"
                    />
                    <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        disabled={busy}
                        onClick={applyPort}
                    >
                        {t('view.settings.integrations.local_api.port_apply')}
                    </Button>
                </div>
            </Field>

            <Field
                label={t('view.settings.integrations.local_api.status_label')}
                description={t(
                    'view.settings.integrations.local_api.port_active_connections',
                    {
                        port: status?.port ?? 8799,
                        count: status?.activeConnections ?? 0
                    }
                )}
            >
                <div className="flex flex-wrap items-center justify-end gap-2">
                    <span className="text-muted-foreground text-sm">
                        {t(
                            `view.settings.integrations.local_api.status.${status?.state ?? 'loading'}`
                        )}
                    </span>
                    <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        disabled={busy}
                        onClick={() => {
                            void runCommand(commands.appLocalApiStatus);
                        }}
                    >
                        <RefreshCwIcon data-icon="inline-start" />
                        {t('common.actions.refresh')}
                    </Button>
                </div>
            </Field>

            <Field
                label={t('view.settings.integrations.local_api.token')}
                description={t(
                    'view.settings.integrations.local_api.token_description'
                )}
            >
                <div className="flex flex-wrap items-center justify-end gap-2">
                    <Input
                        value={status?.token ?? ''}
                        readOnly
                        className="w-64 font-mono"
                    />
                    <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        disabled={busy || !status?.token}
                        onClick={() => void copyToken()}
                    >
                        <CopyIcon data-icon="inline-start" />
                        {t('common.actions.copy')}
                    </Button>
                    <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        disabled={busy}
                        onClick={() => {
                            void runCommand(
                                commands.appLocalApiRotateToken
                            ).then((succeeded) => {
                                if (succeeded) {
                                    toast.success(
                                        t(
                                            'view.settings.integrations.local_api.token_rotated'
                                        )
                                    );
                                }
                            });
                        }}
                    >
                        <KeyRoundIcon data-icon="inline-start" />
                        {t('view.settings.integrations.local_api.rotate_token')}
                    </Button>
                </div>
            </Field>
        </SettingsGroup>
    );
}

function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}
