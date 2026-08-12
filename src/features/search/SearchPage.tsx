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

export function SearchPage({ embedded = false }: { embedded?: boolean } = {}) {
    const { config, filters, results } = useSearchPageController();

    return (
        <PageScaffold embedded={embedded} className="flex-1">
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
                    searched={results.hasUserSearched}
                    onClear={results.handleClearSearch}
                />
                <SearchWorldTabPanel
                    isLoading={results.isWorldLoading}
                    results={results.worldResults}
                    pagination={results.pagination}
                    searched={results.hasWorldSearched}
                    onClear={results.handleClearSearch}
                />
                <SearchAvatarTabPanel
                    isLoading={results.isAvatarLoading}
                    results={results.avatarPageResults}
                    pagination={results.pagination}
                    searched={results.hasAvatarSearched}
                    avatarProviderConfigured={
                        config.avatarProviderEnabled &&
                        Boolean(config.selectedAvatarProvider)
                    }
                    onClear={results.handleClearSearch}
                    onConfigureAvatarProvider={() =>
                        config.setIsAvatarProviderDialogOpen(true)
                    }
                />
                <SearchGroupTabPanel
                    isLoading={results.isGroupLoading}
                    results={results.groupResults}
                    pagination={results.pagination}
                    searched={results.hasGroupSearched}
                    onClear={results.handleClearSearch}
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
