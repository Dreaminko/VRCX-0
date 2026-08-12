import { useState } from 'react';
import { useSearchParams } from 'react-router';

import type { SearchActiveTab } from './searchTypes';

export function useSearchFilters() {
    const [searchParams, setSearchParams] = useSearchParams();
    const requestedTab = searchParams.get('tab');
    const activeTab: SearchActiveTab =
        requestedTab === 'avatar' ||
        requestedTab === 'group' ||
        requestedTab === 'world'
            ? requestedTab
            : 'user';
    const [searchText, setSearchText] = useState('');
    const [searchUserByBio, setSearchUserByBio] = useState(false);
    const [searchUserSortByLastLoggedIn, setSearchUserSortByLastLoggedIn] =
        useState(false);
    const [selectedWorldCategory, setSelectedWorldCategory] = useState('');
    const [includeCommunityLabs, setIncludeCommunityLabs] = useState(false);
    const setActiveTab = (value: string) => {
        if (
            value !== 'avatar' &&
            value !== 'group' &&
            value !== 'user' &&
            value !== 'world'
        ) {
            return;
        }

        const nextSearchParams = new URLSearchParams(searchParams);
        if (value === 'user') {
            nextSearchParams.delete('tab');
        } else {
            nextSearchParams.set('tab', value);
        }
        setSearchParams(nextSearchParams, { replace: true });
    };

    return {
        activeTab,
        includeCommunityLabs,
        searchText,
        searchUserByBio,
        searchUserSortByLastLoggedIn,
        selectedWorldCategory,
        setActiveTab,
        setIncludeCommunityLabs,
        setSearchText,
        setSearchUserByBio,
        setSearchUserSortByLastLoggedIn,
        setSelectedWorldCategory
    };
}
