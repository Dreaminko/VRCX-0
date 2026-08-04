import { PageScaffold } from '@/components/layout/PageScaffold';
import { AvatarProviderSettingsDialog } from '@/components/search/AvatarProviderSettingsDialog';
import { Tabs } from '@/ui/shadcn/tabs';

import { SearchPageToolbar } from './components/SearchPageToolbar';
import {
    SearchAvatarTabPanel,
    SearchGroupTabPanel,
    SearchUserTabPanel,
    SearchWorldTabPanel
} from './components/SearchTabPanels';
import { useSearchPageController } from './useSearchPageController';

export function SearchPage() {
    const { config, filters, results } = useSearchPageController();

    return (
        <PageScaffold className="flex-1">
            <Tabs
                value={filters.activeTab}
                onValueChange={filters.setActiveTab}
                className="flex min-h-0 flex-1 flex-col"
            >
                <SearchPageToolbar
                    activeTab={filters.activeTab}
                    onActiveTabChange={filters.setActiveTab}
                    searchText={filters.searchText}
                    onSearchTextChange={filters.setSearchText}
                    onSearch={results.handleSearch}
                    onClearSearch={results.handleClearSearch}
                    viewOptions={{
                        avatarProviderList: config.avatarProviderList,
                        includeCommunityLabs: filters.includeCommunityLabs,
                        onAvatarProviderChange:
                            config.handleAvatarProviderChange,
                        onIncludeCommunityLabsChange:
                            filters.setIncludeCommunityLabs,
                        onOpenAvatarProviderSettings: () =>
                            config.setIsAvatarProviderDialogOpen(true),
                        onSearchUserByBioChange: filters.setSearchUserByBio,
                        onSearchUserSortByLastLoggedInChange:
                            filters.setSearchUserSortByLastLoggedIn,
                        onWorldCategoryChange:
                            results.handleWorldCategoryChange,
                        searchUserByBio: filters.searchUserByBio,
                        searchUserSortByLastLoggedIn:
                            filters.searchUserSortByLastLoggedIn,
                        selectedAvatarProvider: config.selectedAvatarProvider,
                        selectedWorldCategory: filters.selectedWorldCategory,
                        worldCategories: config.worldCategories
                    }}
                />
                <SearchUserTabPanel
                    isLoading={results.isUserLoading}
                    results={results.userResults}
                    languageOptionsMap={config.languageOptionsMap}
                    pagination={results.pagination}
                />
                <SearchWorldTabPanel
                    isLoading={results.isWorldLoading}
                    results={results.worldResults}
                    pagination={results.pagination}
                />
                <SearchAvatarTabPanel
                    isLoading={results.isAvatarLoading}
                    results={results.avatarPageResults}
                    pagination={results.pagination}
                />
                <SearchGroupTabPanel
                    isLoading={results.isGroupLoading}
                    results={results.groupResults}
                    pagination={results.pagination}
                />
            </Tabs>
            <AvatarProviderSettingsDialog
                open={config.isAvatarProviderDialogOpen}
                onOpenChange={config.setIsAvatarProviderDialogOpen}
                providerList={config.avatarProviderList}
                onConfigSaved={config.applyAvatarProviderConfig}
            />
        </PageScaffold>
    );
}
