import type { FriendRecord } from '@/domain/friends/friendRosterTypes';

export interface MutualFriendNode {
    id: string;
    label: string;
    lastFetchedAt: string | null;
    optedOut: boolean;
    degree: number;
}

export interface MutualFriendLink {
    source: string;
    target: string;
}

export interface MutualFriendGraph {
    nodes: MutualFriendNode[];
    links: MutualFriendLink[];
}

export interface MutualFriendNodeMeta {
    lastFetchedAt: string | null;
    optedOut: boolean;
}

export type MutualFriendSnapshot = Map<string, string[]>;
export type MutualFriendMeta = Map<string, MutualFriendNodeMeta>;

export interface MutualFriendsLayoutSettings {
    layoutIterations: number;
    layoutSpacing: number;
    edgeCurvature: number;
    communitySeparation: number;
}

export type MutualFriendsLayoutSettingKey = keyof MutualFriendsLayoutSettings;

export interface MutualFriendsViewFilters {
    searchQuery: string;
    minDegree: number;
    focusedCommunity: number | null;
}

export interface MutualFriendCommunity {
    index: number;
    size: number;
    color: string;
    label: string;
}

export interface MutualFriendCommunityAssignment {
    communityIndexById: Map<string, number>;
    communities: MutualFriendCommunity[];
}

export interface MutualFriendPickerOption {
    value: string;
    label: string;
    displayLabel: string;
    search: string;
    user: FriendRecord | null;
    degree?: number;
}

export interface MutualFriendsFetchProgress {
    isFetching: boolean;
    processedFriends: number;
    totalFriends: number;
    cancelRequested: boolean;
}

export type MutualFriendsSnapshotStatus =
    | 'idle'
    | 'running'
    | 'ready'
    | 'error';
