import { useQuery } from '@tanstack/react-query';

import { entityQueryPolicies, queryKeys } from '@/lib/entityQueryCache';
import groupProfileRepository from '@/repositories/groupProfileRepository';
import userProfileRepository from '@/repositories/userProfileRepository';
import {
    convertFileUrlToImageUrl,
    userImage
} from '@/services/entityMediaService';
import { normalizeString as normalizeId } from '@/shared/utils/string';
import { useFriendRosterStore } from '@/state/friendRosterStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import type { NotificationActor } from './notificationViewModel';

export function useNotificationActorImage(actor: NotificationActor): string {
    const endpoint = useRuntimeStore((state) => state.auth.currentUserEndpoint);
    const userId =
        actor.kind === 'user' && !actor.imageUrl ? normalizeId(actor.id) : '';
    const rosterFriend = useFriendRosterStore((state) =>
        userId ? (state.friendsById[userId] ?? null) : null
    );
    const rosterImage = rosterFriend ? userImage(rosterFriend, true, 64) : '';
    const groupId =
        actor.kind === 'group' && !actor.imageUrl ? normalizeId(actor.id) : '';

    const profileQuery = useQuery({
        queryKey: queryKeys.user(userId, endpoint),
        queryFn: () => userProfileRepository.getUserProfile({ userId }),
        enabled: Boolean(userId) && !rosterImage,
        staleTime: entityQueryPolicies.userAvatarLookup.staleTime,
        gcTime: entityQueryPolicies.userAvatarLookup.gcTime,
        retry: entityQueryPolicies.userAvatarLookup.retry,
        refetchOnWindowFocus:
            entityQueryPolicies.userAvatarLookup.refetchOnWindowFocus
    });

    const groupQuery = useQuery({
        queryKey: queryKeys.group(groupId, false, endpoint),
        queryFn: () =>
            groupProfileRepository.fetchGroupProfile({
                groupId,
                includeRoles: false
            }),
        enabled: Boolean(groupId),
        staleTime: entityQueryPolicies.group.staleTime,
        gcTime: entityQueryPolicies.group.gcTime,
        retry: entityQueryPolicies.group.retry,
        refetchOnWindowFocus: entityQueryPolicies.group.refetchOnWindowFocus
    });

    if (actor.kind === 'group') {
        return (
            actor.imageUrl ||
            convertFileUrlToImageUrl(groupQuery.data?.iconUrl, 64)
        );
    }
    if (actor.kind !== 'user') {
        return '';
    }
    if (actor.imageUrl) {
        return actor.imageUrl;
    }
    if (rosterImage) {
        return rosterImage;
    }
    return profileQuery.data ? userImage(profileQuery.data, true, 64) : '';
}
