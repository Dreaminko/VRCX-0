import { useQuery } from '@tanstack/react-query';

import { entityQueryPolicies, queryKeys } from '@/lib/entityQueryCache';
import userProfileRepository from '@/repositories/userProfileRepository';
import { userImage } from '@/services/entityMediaService';
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

    if (actor.kind !== 'user') {
        return actor.kind === 'group' ? actor.imageUrl : '';
    }
    if (actor.imageUrl) {
        return actor.imageUrl;
    }
    if (rosterImage) {
        return rosterImage;
    }
    return profileQuery.data ? userImage(profileQuery.data, true, 64) : '';
}
