import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/utils';

import { MUTUAL_GRAPH_MIN_DEGREE_LIMITS } from '../../mutual-friends/mutualFriendsFilters';
import type { MutualFriendCommunity } from '../../mutual-friends/mutualFriendsTypes';
import { CommitSlider } from './CommitSlider';
import { MutualFriendsSurface } from './MutualFriendsSurface';

const VISIBLE_COMMUNITY_LIMIT = 6;

export function MutualFriendsLegend({
    communities,
    focusedCommunity,
    isolatedCount,
    minDegree,
    onMinDegreeChange,
    onToggleFocusedCommunity
}: {
    communities: MutualFriendCommunity[];
    focusedCommunity: number | null;
    isolatedCount: number;
    minDegree: number;
    onMinDegreeChange: (value: number) => void;
    onToggleFocusedCommunity: (communityIndex: number) => void;
}) {
    const { t } = useTranslation();
    const namedCommunities = communities.filter(
        (community) => community.size > 1
    );
    const visibleCommunities = namedCommunities.slice(
        0,
        VISIBLE_COMMUNITY_LIMIT
    );
    const hiddenCommunityCount =
        namedCommunities.length - visibleCommunities.length;

    return (
        <MutualFriendsSurface className="animate-in fade-in-0 slide-in-from-bottom-2 pointer-events-auto absolute bottom-3 left-3 z-10 w-64 p-3 duration-200 ease-out">
            <div className="flex items-baseline justify-between">
                <span className="text-foreground text-xs font-medium">
                    {t('view.charts.mutual_friend.legend.circles')}
                </span>
                <span className="text-muted-foreground text-xs tabular-nums">
                    {namedCommunities.length}
                </span>
            </div>

            <div className="mt-2 flex flex-col gap-0.5">
                {visibleCommunities.map((community) => {
                    const isFocused = focusedCommunity === community.index;
                    return (
                        <button
                            key={community.index}
                            type="button"
                            onClick={() =>
                                onToggleFocusedCommunity(community.index)
                            }
                            className={cn(
                                'flex w-full items-center gap-2 rounded-md px-1.5 py-1 text-left transition-[background-color,opacity] duration-150 ease-out',
                                'hover:bg-foreground/5 active:translate-y-px',
                                isFocused
                                    ? 'bg-foreground/10'
                                    : focusedCommunity !== null
                                      ? 'opacity-50'
                                      : ''
                            )}
                        >
                            <span
                                className="size-2.5 shrink-0 rounded-full"
                                style={{ backgroundColor: community.color }}
                            />
                            <span className="min-w-0 flex-1 truncate text-xs">
                                {community.label}
                            </span>
                            <span className="text-muted-foreground text-xs tabular-nums">
                                {community.size}
                            </span>
                        </button>
                    );
                })}
                {hiddenCommunityCount > 0 ? (
                    <span className="text-muted-foreground px-1.5 py-1 text-xs">
                        {t('view.charts.mutual_friend.legend.more_circles', {
                            count: hiddenCommunityCount
                        })}
                    </span>
                ) : null}
            </div>

            <div className="bg-border my-2.5 h-px" />

            <ul className="text-muted-foreground flex flex-col gap-1.5 text-xs">
                <li className="flex items-center gap-2">
                    <span className="flex w-4 shrink-0 items-center justify-center gap-0.5">
                        <span className="bg-muted-foreground/70 size-1 rounded-full" />
                        <span className="bg-muted-foreground/70 size-2.5 rounded-full" />
                    </span>
                    {t('view.charts.mutual_friend.legend.size_means_degree')}
                </li>
            </ul>

            <div className="mt-3 text-xs">
                <CommitSlider
                    label={t('view.charts.mutual_friend.legend.min_degree')}
                    help={
                        isolatedCount > 0
                            ? t(
                                  'view.charts.mutual_friend.legend.isolated_nodes',
                                  { count: isolatedCount }
                              )
                            : undefined
                    }
                    min={MUTUAL_GRAPH_MIN_DEGREE_LIMITS.min}
                    max={MUTUAL_GRAPH_MIN_DEGREE_LIMITS.max}
                    step={1}
                    value={minDegree}
                    onCommit={onMinDegreeChange}
                />
            </div>
        </MutualFriendsSurface>
    );
}
