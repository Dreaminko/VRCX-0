import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import {
    profileBackupErrorKey,
    profileRestoreFailureKey
} from '@/services/profileBackupI18n';
import {
    getProfileBackupSettings,
    runManualProfileBackup,
    setProfileBackupSettings,
    validateProfileRestore,
    type ProfileBackupSettings
} from '@/services/profileBackupService';
import {
    openFileSelectorDialog,
    openFolderSelectorDialog
} from '@/services/shellIntegrationService';
import { useModalStore } from '@/state/modalStore';
import { useProfileBackupStore } from '@/state/profileBackupStore';

type NumericProfileBackupSetting = 'autoIntervalDays' | 'autoRetainExtra';

function clampInteger(value: unknown, min: number, max: number): number {
    const parsed = Number.parseInt(String(value), 10);
    if (!Number.isFinite(parsed)) {
        return min;
    }
    return Math.min(max, Math.max(min, parsed));
}

export function useProfileBackupSettings(enabled: boolean) {
    const { t } = useTranslation();
    const confirm = useModalStore((state) => state.confirm);
    const openRestoreDialog = useProfileBackupStore(
        (state) => state.openRestoreDialog
    );
    const applyStatus = useProfileBackupStore((state) => state.applyStatus);
    const lastOutcome = useProfileBackupStore(
        (state) => state.status.lastOutcome
    );
    const [settings, setSettings] = useState<ProfileBackupSettings | null>(
        null
    );
    const [numericDrafts, setNumericDrafts] = useState<
        Partial<Record<NumericProfileBackupSetting, string>>
    >({});
    const persistedSettingsRef = useRef<ProfileBackupSettings | null>(null);
    const lastRefreshedAutoOutcomeRevisionRef = useRef(-1);
    const settingsRequestRevisionRef = useRef(0);
    const manualBackupRunningRef = useRef(false);
    const autoEnabledTogglingRef = useRef(false);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [startingManualBackup, setStartingManualBackup] = useState(false);
    const [validatingRestore, setValidatingRestore] = useState(false);

    useEffect(() => {
        setNumericDrafts({});
        if (!enabled) {
            lastRefreshedAutoOutcomeRevisionRef.current = -1;
            return undefined;
        }
        lastRefreshedAutoOutcomeRevisionRef.current =
            useProfileBackupStore.getState().status.lastOutcome?.revision ?? -1;
        let active = true;
        const requestRevision = ++settingsRequestRevisionRef.current;
        setLoading(true);
        getProfileBackupSettings()
            .then((loaded) => {
                if (
                    !active ||
                    requestRevision !== settingsRequestRevisionRef.current
                ) {
                    return;
                }
                persistedSettingsRef.current = loaded;
                setSettings(loaded);
            })
            .catch(() => {
                if (
                    active &&
                    requestRevision === settingsRequestRevisionRef.current
                ) {
                    toast.error(t('profile_backup.settings_load_failed'));
                }
            })
            .finally(() => {
                if (
                    active &&
                    requestRevision === settingsRequestRevisionRef.current
                ) {
                    setLoading(false);
                }
            });

        return () => {
            active = false;
        };
    }, [enabled, t]);

    useEffect(() => {
        if (
            !enabled ||
            !lastOutcome?.succeeded ||
            lastOutcome.kind !== 'auto' ||
            lastOutcome.revision <= lastRefreshedAutoOutcomeRevisionRef.current
        ) {
            return undefined;
        }
        let active = true;
        const requestRevision = ++settingsRequestRevisionRef.current;
        getProfileBackupSettings()
            .then((loaded) => {
                if (
                    !active ||
                    requestRevision !== settingsRequestRevisionRef.current
                ) {
                    return;
                }
                lastRefreshedAutoOutcomeRevisionRef.current =
                    lastOutcome.revision;
                persistedSettingsRef.current = loaded;
                setSettings(loaded);
            })
            .catch(() => {
                if (
                    active &&
                    requestRevision === settingsRequestRevisionRef.current
                ) {
                    toast.error(t('profile_backup.settings_load_failed'));
                }
            });

        return () => {
            active = false;
        };
    }, [enabled, lastOutcome, t]);

    async function persistSettings(next: ProfileBackupSettings) {
        const previous = persistedSettingsRef.current;
        setSettings(next);
        setSaving(true);
        try {
            const saved = await setProfileBackupSettings(next);
            persistedSettingsRef.current = saved;
            setSettings(saved);
            return true;
        } catch {
            setSettings(previous);
            toast.error(t('profile_backup.settings_save_failed'));
            return false;
        } finally {
            setSaving(false);
        }
    }

    function numericDraftValue(key: NumericProfileBackupSetting): string {
        const draft = numericDrafts[key];
        if (draft !== undefined) {
            return draft;
        }
        if (!settings) {
            return key === 'autoIntervalDays' ? '7' : '3';
        }
        return key === 'autoIntervalDays'
            ? String(settings.autoIntervalDays)
            : String(settings.autoRetainExtra + 1);
    }

    function setNumericDraft(key: NumericProfileBackupSetting, value: string) {
        setNumericDrafts((drafts) => ({ ...drafts, [key]: value }));
    }

    async function commitNumericDraft(key: NumericProfileBackupSetting) {
        const draft = numericDrafts[key];
        if (draft === undefined) {
            return;
        }
        if (!settings || saving) {
            return;
        }
        setNumericDrafts((drafts) => {
            const next = { ...drafts };
            delete next[key];
            return next;
        });
        const next =
            key === 'autoIntervalDays'
                ? { ...settings, autoIntervalDays: clampInteger(draft, 1, 30) }
                : {
                      ...settings,
                      autoRetainExtra: clampInteger(draft, 2, 6) - 1
                  };
        await persistSettings(next);
    }

    async function selectBackupFolder(defaultPath: string): Promise<string> {
        try {
            return await openFolderSelectorDialog(defaultPath);
        } catch {
            toast.error(t('profile_backup.folder_selection_failed'));
            return '';
        }
    }

    async function setAutoEnabled(enabled: boolean) {
        if (
            !settings ||
            saving ||
            enabled === settings.autoEnabled ||
            autoEnabledTogglingRef.current
        ) {
            return;
        }
        autoEnabledTogglingRef.current = true;
        try {
            let nextSettings = settings;
            if (enabled && !settings.autoTargetDir) {
                const selected = await selectBackupFolder('');
                if (!selected) {
                    return;
                }
                nextSettings = { ...settings, autoTargetDir: selected };
            }
            if (enabled) {
                const result = await confirm({
                    title: t('profile_backup.unencrypted_warning_title'),
                    description: t('profile_backup.unencrypted_warning'),
                    confirmText: t('profile_backup.enable_automatic'),
                    cancelText: t('common.actions.cancel')
                });
                if (!result.ok) {
                    return;
                }
            }
            await persistSettings({ ...nextSettings, autoEnabled: enabled });
        } finally {
            autoEnabledTogglingRef.current = false;
        }
    }

    async function chooseAutomaticBackupFolder() {
        if (!settings || saving) {
            return;
        }
        const selected = await selectBackupFolder(settings.autoTargetDir);
        if (!selected) {
            return;
        }
        await persistSettings({ ...settings, autoTargetDir: selected });
    }

    async function startManualBackup() {
        if (
            !settings ||
            startingManualBackup ||
            manualBackupRunningRef.current
        ) {
            return;
        }
        manualBackupRunningRef.current = true;
        setStartingManualBackup(true);
        try {
            const targetDir = await selectBackupFolder(settings.autoTargetDir);
            if (!targetDir) {
                return;
            }
            const result = await confirm({
                title: t('profile_backup.unencrypted_warning_title'),
                description: t('profile_backup.unencrypted_warning'),
                confirmText: t('profile_backup.backup_now_confirm'),
                cancelText: t('common.actions.cancel')
            });
            if (!result.ok) {
                return;
            }
            const outcome = await runManualProfileBackup(targetDir);
            applyStatus(outcome.status);
            if (!outcome.accepted) {
                toast.error(
                    t(
                        outcome.error
                            ? profileBackupErrorKey(outcome.error.code)
                            : 'profile_backup.error.unknown'
                    )
                );
            }
        } catch {
            toast.error(t('profile_backup.backup_start_failed'));
        } finally {
            manualBackupRunningRef.current = false;
            setStartingManualBackup(false);
        }
    }

    async function selectBackupToRestore() {
        if (validatingRestore) {
            return;
        }
        let path = '';
        try {
            path = await openFileSelectorDialog(
                settings?.autoTargetDir || '',
                '.vrcx0backup',
                `${t('profile_backup.file_filter')} (*.vrcx0backup;*.zip)|*.vrcx0backup;*.zip`
            );
        } catch {
            toast.error(t('profile_backup.file_selection_failed'));
            return;
        }
        if (!path) {
            return;
        }

        setValidatingRestore(true);
        const toastId = toast.loading(t('profile_backup.validating_restore'));
        try {
            const outcome = await validateProfileRestore(path);
            if (!outcome.validation) {
                const errorKey = outcome.failure
                    ? profileRestoreFailureKey(outcome.failure.code)
                    : 'profile_backup.error.unknown';
                toast.error(t(errorKey));
                return;
            }
            openRestoreDialog(path, outcome.validation);
        } catch {
            toast.error(t('profile_backup.restore_validation_failed'));
        } finally {
            toast.dismiss(toastId);
            setValidatingRestore(false);
        }
    }

    return {
        settings,
        loading,
        saving,
        startingManualBackup,
        validatingRestore,
        numericDraftValue,
        setNumericDraft,
        commitNumericDraft,
        setAutoEnabled,
        chooseAutomaticBackupFolder,
        startManualBackup,
        selectBackupToRestore
    };
}
