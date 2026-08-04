import type {
    AvatarProfileRecord,
    UserProfileRecord,
    WorldProfileRecord
} from '@/domain/entities/profileEntities';
import type { SearchGroupJson } from '@/repositories/vrchatSearchRepository';

import type {
    buildAvatarSearchRequest,
    buildGroupSearchRequest,
    buildUserSearchRequest,
    buildWorldSearchRequest,
    WorldSearchCategory
} from './searchRequests';

export type SearchActiveTab = 'avatar' | 'group' | 'user' | 'world';
export type SearchWorldCategory = WorldSearchCategory & {
    index: string | number;
    name?: string;
};

export type SearchUserResult = UserProfileRecord;
export type SearchWorldResult = WorldProfileRecord;
export type SearchGroupResult = SearchGroupJson;
export type SearchAvatarResult = AvatarProfileRecord;

export type AvatarSearchRequest = ReturnType<typeof buildAvatarSearchRequest>;
export type GroupSearchRequest = ReturnType<typeof buildGroupSearchRequest>;
export type UserSearchRequest = ReturnType<typeof buildUserSearchRequest>;
export type WorldSearchRequest = ReturnType<typeof buildWorldSearchRequest>;

export type SearchPaginationState = {
    show: boolean;
    prevDisabled: boolean;
    nextDisabled: boolean;
    onPrev: () => void;
    onNext: () => void;
};
