export type FeedLiveEntryPayload = Record<string, unknown> & {
    id?: string | number;
    rowId?: string | number;
    row_id?: string | number;
    sourceRank?: string | number;
    source_rank?: string | number;
    type?: string;
    created_at?: string;
    createdAt?: string;
    userId?: string;
    senderUserId?: string;
    ownerUserId?: string;
    displayName?: string;
    details?: Record<string, unknown>;
    location?: string;
    message?: string;
    groupName?: string;
    previousLocation?: string;
    time?: string | number;
    worldId?: string;
    worldName?: string;
    displayLocation?: string;
    avatarName?: string;
    currentAvatarImageUrl?: string;
    currentAvatarTags?: string[];
    currentAvatarThumbnailImageUrl?: string;
    ownerId?: string;
    previousAvatarName?: string;
    previousCurrentAvatarImageUrl?: string;
    previousCurrentAvatarTags?: string[];
    previousCurrentAvatarThumbnailImageUrl?: string;
    previousOwnerId?: string;
};

export type FeedLiveAvatarEntryPayload = FeedLiveEntryPayload & {
    type?: 'Avatar' | string;
    avatarName?: string;
    created_at?: string;
    currentAvatarImageUrl?: string;
    currentAvatarTags?: string[];
    currentAvatarThumbnailImageUrl?: string;
    displayName?: string;
    ownerId?: string;
    previousAvatarName?: string;
    previousCurrentAvatarImageUrl?: string;
    previousCurrentAvatarTags?: string[];
    previousCurrentAvatarThumbnailImageUrl?: string;
    previousOwnerId?: string;
    userId?: string;
};

export type FeedLiveLocationEntryPayload = FeedLiveEntryPayload & {
    type?: 'GPS' | string;
    created_at?: string;
    displayLocation?: string;
    displayName?: string;
    groupName?: string;
    location?: string;
    previousLocation?: string;
    time?: string;
    userId?: string;
    worldId?: string;
    worldName?: string;
};

export type FeedLiveEntry = {
    sequence: number;
    ownerUserId?: string;
    entry: FeedLiveEntryPayload;
};

export type FeedEntryPatchInput = Record<string, unknown> & {
    displayName?: string;
    worldName?: string;
    displayLocation?: string;
};

export type FeedEntryPatch = Partial<{
    displayName: string;
    worldName: string;
    displayLocation: string;
}>;
