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
    user_id?: string;
    senderUserId?: string;
    ownerUserId?: string;
    owner_user_id?: string;
    displayName?: string;
    display_name?: string;
    details?: Record<string, unknown>;
    location?: string;
    message?: string;
    groupName?: string;
    group_name?: string;
    previousLocation?: string;
    previous_location?: string;
    time?: string | number;
    worldId?: string;
    worldName?: string;
    world_name?: string;
    displayLocation?: string;
    avatarName?: string;
    avatar_name?: string;
    currentAvatarImageUrl?: string;
    current_avatar_image_url?: string;
    currentAvatarTags?: string[];
    current_avatar_tags?: string[];
    currentAvatarThumbnailImageUrl?: string;
    current_avatar_thumbnail_image_url?: string;
    ownerId?: string;
    owner_id?: string;
    previousAvatarName?: string;
    previous_avatar_name?: string;
    previousCurrentAvatarImageUrl?: string;
    previous_current_avatar_image_url?: string;
    previousCurrentAvatarTags?: string[];
    previous_current_avatar_tags?: string[];
    previousCurrentAvatarThumbnailImageUrl?: string;
    previous_current_avatar_thumbnail_image_url?: string;
    previousOwnerId?: string;
    previous_owner_id?: string;
    statusDescription?: string;
    status_description?: string;
    previousStatus?: string;
    previous_status?: string;
    previousStatusDescription?: string;
    previous_status_description?: string;
    previousBio?: string;
    previous_bio?: string;
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

export type FeedLivePatch = {
    sequence: number;
    id: string;
    fields: FeedEntryPatchInput;
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
