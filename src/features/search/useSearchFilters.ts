import { useState } from 'react';

import type { SearchActiveTab } from './searchTypes';

export function useSearchFilters() {
    const [activeTab, setActiveTab] = useState<SearchActiveTab>('user');
    const [searchText, setSearchText] = useState('');
    const [searchUserByBio, setSearchUserByBio] = useState(false);
    const [searchUserSortByLastLoggedIn, setSearchUserSortByLastLoggedIn] =
        useState(false);
    const [selectedWorldCategory, setSelectedWorldCategory] = useState('');
    const [includeCommunityLabs, setIncludeCommunityLabs] = useState(false);

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
