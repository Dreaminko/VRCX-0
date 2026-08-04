import { useEffect, useState } from 'react';

import configRepository from '@/repositories/configRepository';
import { onPreferenceChanged } from '@/shared/events/preferenceEvents';

import { parseConfigArray } from './friendsLocationsConfig';
import {
    DEFAULT_FRIENDS_LOCATIONS_DENSITY,
    sanitizeFriendsLocationsDensity
} from './friendsLocationsDensity';

type FriendsLocationsSidebarFavoritePrefs = {
    isDivideByGroup: boolean;
    selectedGroups: string[];
    groupOrder: string[];
};

export function useFriendsLocationsPreferences() {
    const [preferencesReady, setPreferencesReady] = useState(false);
    const [showSameInstanceInOnline, setShowSameInstanceInOnline] =
        useState(false);
    const [showCurrentUserInSameInstance, setShowCurrentUserInSameInstance] =
        useState(true);
    const [density, setDensity] = useState(DEFAULT_FRIENDS_LOCATIONS_DENSITY);
    const [sidebarFavoritePrefs, setSidebarFavoritePrefs] =
        useState<FriendsLocationsSidebarFavoritePrefs>({
            isDivideByGroup: false,
            selectedGroups: [],
            groupOrder: []
        });
    const [sidebarSortMethods, setSidebarSortMethods] = useState<string[]>([
        'Sort by Status',
        'Sort Alphabetically',
        ''
    ]);

    useEffect(() => {
        let active = true;

        Promise.all([
            configRepository.getString(
                'FriendLocationDensity',
                DEFAULT_FRIENDS_LOCATIONS_DENSITY
            ),
            configRepository.getBool('FriendLocationShowSameInstance', false),
            configRepository.getBool('isShowCurrentUserInSameInstance', true),
            configRepository.getBool('isSidebarDivideByFriendGroup', false),
            configRepository.getString('sidebarFavoriteGroups', '[]'),
            configRepository.getString('sidebarFavoriteGroupOrder', '[]'),
            configRepository.getString('sidebarSortMethod1', 'Sort by Status'),
            configRepository.getString(
                'sidebarSortMethod2',
                'Sort Alphabetically'
            ),
            configRepository.getString('sidebarSortMethod3', '')
        ])
            .then(
                ([
                    nextDensity,
                    nextShowSameInstance,
                    nextShowCurrentUser,
                    nextDivideByGroup,
                    nextSelectedGroups,
                    nextGroupOrder,
                    nextSortMethod1,
                    nextSortMethod2,
                    nextSortMethod3
                ]) => {
                    if (!active) {
                        return;
                    }

                    setDensity(sanitizeFriendsLocationsDensity(nextDensity));
                    setShowSameInstanceInOnline(Boolean(nextShowSameInstance));
                    setShowCurrentUserInSameInstance(
                        Boolean(nextShowCurrentUser)
                    );
                    setSidebarFavoritePrefs({
                        isDivideByGroup: Boolean(nextDivideByGroup),
                        selectedGroups: parseConfigArray(nextSelectedGroups),
                        groupOrder: parseConfigArray(nextGroupOrder)
                    });
                    setSidebarSortMethods([
                        nextSortMethod1 || '',
                        nextSortMethod2 || '',
                        nextSortMethod3 || ''
                    ]);
                    setPreferencesReady(true);
                }
            )
            .catch(() => {
                if (active) {
                    setPreferencesReady(true);
                }
            });

        return () => {
            active = false;
        };
    }, []);

    useEffect(() => {
        let active = true;
        const unsubscribe = onPreferenceChanged(
            [
                'isSidebarDivideByFriendGroup',
                'isShowCurrentUserInSameInstance',
                'sidebarFavoriteGroups',
                'sidebarFavoriteGroupOrder',
                'sidebarSortMethod1',
                'sidebarSortMethod2',
                'sidebarSortMethod3'
            ],
            async () => {
                try {
                    const [
                        nextDivideByGroup,
                        nextShowCurrentUser,
                        nextSelectedGroups,
                        nextGroupOrder,
                        nextSortMethod1,
                        nextSortMethod2,
                        nextSortMethod3
                    ] = await Promise.all([
                        configRepository.getBool(
                            'isSidebarDivideByFriendGroup',
                            false
                        ),
                        configRepository.getBool(
                            'isShowCurrentUserInSameInstance',
                            true
                        ),
                        configRepository.getString(
                            'sidebarFavoriteGroups',
                            '[]'
                        ),
                        configRepository.getString(
                            'sidebarFavoriteGroupOrder',
                            '[]'
                        ),
                        configRepository.getString(
                            'sidebarSortMethod1',
                            'Sort by Status'
                        ),
                        configRepository.getString(
                            'sidebarSortMethod2',
                            'Sort Alphabetically'
                        ),
                        configRepository.getString('sidebarSortMethod3', '')
                    ]);
                    if (active) {
                        setShowCurrentUserInSameInstance(
                            Boolean(nextShowCurrentUser)
                        );
                        setSidebarFavoritePrefs({
                            isDivideByGroup: Boolean(nextDivideByGroup),
                            selectedGroups:
                                parseConfigArray(nextSelectedGroups),
                            groupOrder: parseConfigArray(nextGroupOrder)
                        });
                        setSidebarSortMethods([
                            nextSortMethod1 || '',
                            nextSortMethod2 || '',
                            nextSortMethod3 || ''
                        ]);
                    }
                } catch {
                    // no-op
                }
            }
        );

        return () => {
            active = false;
            unsubscribe();
        };
    }, []);

    function changeShowSameInstanceInOnline(value: unknown) {
        const nextValue = Boolean(value);
        setShowSameInstanceInOnline(nextValue);
        configRepository.setBool('FriendLocationShowSameInstance', nextValue);
    }

    function changeDensityPreference(value: unknown) {
        const nextValue = sanitizeFriendsLocationsDensity(value);
        setDensity(nextValue);
        configRepository.setString('FriendLocationDensity', nextValue);
    }

    return {
        changeDensityPreference,
        changeShowSameInstanceInOnline,
        density,
        preferencesReady,
        showCurrentUserInSameInstance,
        showSameInstanceInOnline,
        sidebarFavoritePrefs,
        sidebarSortMethods
    };
}
