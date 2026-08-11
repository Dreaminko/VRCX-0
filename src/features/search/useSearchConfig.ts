import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { AVATAR_SEARCH_PROVIDER_PREFERENCE_KEYS } from '@/repositories/avatarSearchProviderRepository';
import avatarSearchProviderRepository from '@/repositories/avatarSearchProviderRepository';
import type { AvatarSearchProviderConfig } from '@/repositories/avatarSearchProviderRepository';
import { onPreferenceChanged } from '@/shared/events/preferenceEvents';
import {
    type LanguageOption,
    normalizeLanguageOptionsFromConfig
} from '@/shared/utils/userLanguage';
import { useVrchatConfigStore } from '@/state/vrchatConfigStore';

import { emptyArray } from './searchResults';
import type { SearchWorldCategory } from './searchTypes';

function isWorldCategory(value: unknown): value is SearchWorldCategory {
    return Boolean(
        value &&
        typeof value === 'object' &&
        'index' in value &&
        value.index !== null &&
        value.index !== undefined
    );
}

export function useSearchConfig() {
    const { t } = useTranslation();
    const vrchatConfig = useVrchatConfigStore((state) => state.snapshot);
    const worldCategories = useMemo(
        () =>
            emptyArray(vrchatConfig?.dynamicWorldRows).filter(isWorldCategory),
        [vrchatConfig]
    );
    const languageOptionsMap = useMemo(
        () =>
            new Map(
                normalizeLanguageOptionsFromConfig(vrchatConfig).map(
                    (option): [string, LanguageOption] => [option.key, option]
                )
            ),
        [vrchatConfig]
    );
    const [avatarProviderEnabled, setAvatarProviderEnabled] = useState(false);
    const [avatarProviderList, setAvatarProviderList] = useState<string[]>([]);
    const [selectedAvatarProvider, setSelectedAvatarProvider] = useState('');
    const [isAvatarProviderDialogOpen, setIsAvatarProviderDialogOpen] =
        useState(false);

    function applyAvatarProviderConfig(config: AvatarSearchProviderConfig) {
        setAvatarProviderEnabled(config.enabled);
        setAvatarProviderList(config.providerList);
        setSelectedAvatarProvider(config.selectedProvider || '');
    }

    useEffect(() => {
        let active = true;
        const unsubscribe = onPreferenceChanged(
            AVATAR_SEARCH_PROVIDER_PREFERENCE_KEYS,
            () => {
                avatarSearchProviderRepository
                    .getConfig()
                    .then((config) => {
                        if (active) {
                            applyAvatarProviderConfig(config);
                        }
                    })
                    .catch((error: unknown) => {
                        console.warn(
                            'Failed to refresh avatar providers:',
                            error
                        );
                    });
            }
        );

        avatarSearchProviderRepository
            .getConfig()
            .then((config) => {
                if (!active) {
                    return;
                }

                applyAvatarProviderConfig(config);
            })
            .catch((error: unknown) => {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : t('view.search.toast.failed_to_load_avatar_providers')
                );
            });

        return () => {
            active = false;
            unsubscribe();
        };
    }, [t]);

    function handleAvatarProviderChange(provider: string | null) {
        const nextProvider = provider ?? '';
        setSelectedAvatarProvider(nextProvider);
        avatarSearchProviderRepository
            .saveSelectedProvider(nextProvider)
            .catch((error: unknown) => {
                toast.error(
                    error instanceof Error
                        ? error.message
                        : t('view.search.toast.failed_to_save_avatar_provider')
                );
            });
    }

    return {
        applyAvatarProviderConfig,
        avatarProviderEnabled,
        avatarProviderList,
        handleAvatarProviderChange,
        isAvatarProviderDialogOpen,
        languageOptionsMap,
        selectedAvatarProvider,
        setIsAvatarProviderDialogOpen,
        worldCategories
    };
}
