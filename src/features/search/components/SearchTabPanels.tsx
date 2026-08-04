import { SearchPagination } from '@/components/search/SearchPagination';
import type { LanguageOption } from '@/shared/utils/userLanguage';
import { usePreferencesStore } from '@/state/preferencesStore';
import { TabsContent } from '@/ui/shadcn/tabs';

import type {
    SearchAvatarResult,
    SearchGroupResult,
    SearchPaginationState,
    SearchUserResult,
    SearchWorldResult
} from '../searchTypes';
import {
    AvatarCard,
    GroupRow,
    SearchEmptyState,
    SearchLoadingState,
    UserRow,
    WorldCard
} from './SearchResultParts';

export function SearchUserTabPanel({
    isLoading,
    results,
    languageOptionsMap,
    pagination
}: {
    isLoading: boolean;
    results: SearchUserResult[];
    languageOptionsMap: ReadonlyMap<string, LanguageOption>;
    pagination: SearchPaginationState;
}) {
    const randomUserColours = usePreferencesStore(
        (state) => state.randomUserColours
    );
    const isDarkMode =
        typeof document !== 'undefined' &&
        document.documentElement.classList.contains('dark');

    return (
        <TabsContent
            value="user"
            keepMounted
            className="m-0 flex min-h-0 flex-1 flex-col data-hidden:hidden"
        >
            <div className="flex min-h-0 flex-col" style={{ flex: 9 }}>
                <div className="min-h-0 flex-1 overflow-y-auto">
                    {isLoading ? (
                        <SearchLoadingState />
                    ) : results.length > 0 ? (
                        <div className="grid [grid-template-columns:repeat(auto-fill,minmax(min(280px,100%),1fr))] gap-3">
                            {results.map((user) => (
                                <UserRow
                                    key={user.id}
                                    user={user}
                                    randomUserColours={randomUserColours}
                                    isDarkMode={isDarkMode}
                                    languageOptionsMap={languageOptionsMap}
                                />
                            ))}
                        </div>
                    ) : (
                        <SearchEmptyState />
                    )}
                </div>
            </div>
            <SearchPagination
                show={pagination.show}
                prevDisabled={pagination.prevDisabled}
                nextDisabled={pagination.nextDisabled}
                onPrev={pagination.onPrev}
                onNext={pagination.onNext}
            />
        </TabsContent>
    );
}

export function SearchWorldTabPanel({
    isLoading,
    results,
    pagination
}: {
    isLoading: boolean;
    results: SearchWorldResult[];
    pagination: SearchPaginationState;
}) {
    return (
        <TabsContent
            value="world"
            keepMounted
            className="m-0 flex min-h-0 flex-1 flex-col data-hidden:hidden"
        >
            <div className="flex min-h-0 flex-col" style={{ flex: 9 }}>
                <div className="min-h-0 flex-1 overflow-y-auto">
                    {isLoading ? (
                        <SearchLoadingState />
                    ) : results.length > 0 ? (
                        <div className="grid [grid-template-columns:repeat(auto-fill,minmax(220px,1fr))] gap-4">
                            {results.map((world) => (
                                <WorldCard key={world.id} world={world} />
                            ))}
                        </div>
                    ) : (
                        <SearchEmptyState />
                    )}
                </div>
            </div>
            <SearchPagination
                show={pagination.show}
                prevDisabled={pagination.prevDisabled}
                nextDisabled={pagination.nextDisabled}
                onPrev={pagination.onPrev}
                onNext={pagination.onNext}
            />
        </TabsContent>
    );
}

export function SearchAvatarTabPanel({
    isLoading,
    results,
    pagination
}: {
    isLoading: boolean;
    results: SearchAvatarResult[];
    pagination: SearchPaginationState;
}) {
    return (
        <TabsContent
            value="avatar"
            keepMounted
            className="m-0 flex min-h-0 flex-1 flex-col data-hidden:hidden"
        >
            <div className="flex min-h-0 flex-col" style={{ flex: 9 }}>
                <div className="min-h-0 flex-1 overflow-y-auto">
                    {isLoading ? (
                        <SearchLoadingState />
                    ) : results.length > 0 ? (
                        <div className="grid [grid-template-columns:repeat(auto-fill,minmax(220px,1fr))] gap-4">
                            {results.map((avatar) => (
                                <AvatarCard key={avatar.id} avatar={avatar} />
                            ))}
                        </div>
                    ) : (
                        <SearchEmptyState />
                    )}
                </div>
            </div>
            <SearchPagination
                show={pagination.show}
                prevDisabled={pagination.prevDisabled}
                nextDisabled={pagination.nextDisabled}
                onPrev={pagination.onPrev}
                onNext={pagination.onNext}
            />
        </TabsContent>
    );
}

export function SearchGroupTabPanel({
    isLoading,
    results,
    pagination
}: {
    isLoading: boolean;
    results: SearchGroupResult[];
    pagination: SearchPaginationState;
}) {
    return (
        <TabsContent
            value="group"
            keepMounted
            className="m-0 flex min-h-0 flex-1 flex-col data-hidden:hidden"
        >
            <div className="min-h-0 flex-1 overflow-y-auto" style={{ flex: 9 }}>
                {isLoading ? (
                    <SearchLoadingState />
                ) : results.length > 0 ? (
                    <div className="grid [grid-template-columns:repeat(auto-fill,minmax(min(280px,100%),1fr))] gap-3">
                        {results.map((group) => (
                            <GroupRow key={group.id} group={group} />
                        ))}
                    </div>
                ) : (
                    <SearchEmptyState />
                )}
            </div>
            <SearchPagination
                show={pagination.show}
                prevDisabled={pagination.prevDisabled}
                nextDisabled={pagination.nextDisabled}
                onPrev={pagination.onPrev}
                onNext={pagination.onNext}
            />
        </TabsContent>
    );
}
