import mutualGraphPersistenceRepository from '@/repositories/mutualGraphPersistenceRepository';
import { executeWithBackoff } from '@/shared/utils/retry';

import { isValidMutualFriendId } from './mutualFriendsSettings';

interface FetchMutualFriendIdsOptions {
    rateLimiter?: { wait: () => Promise<void> } | null;
    isCancelled?: () => boolean;
}

export async function fetchMutualFriendIds(
    friendId: string,
    {
        rateLimiter = null,
        isCancelled = () => false
    }: FetchMutualFriendIdsOptions = {}
) {
    const collected: string[] = [];
    let offset = 0;

    while (true) {
        if (isCancelled()) {
            break;
        }
        if (rateLimiter) {
            await rateLimiter.wait();
        }
        if (isCancelled()) {
            break;
        }

        const response = await executeWithBackoff(
            () => {
                if (isCancelled()) {
                    throw new Error('cancelled');
                }
                return mutualGraphPersistenceRepository.getMutualFriends({
                    friendId,
                    offset,
                    n: 100
                });
            },
            {
                maxRetries: 4,
                baseDelay: 500,
                shouldRetry: (error: unknown) =>
                    (error as { status?: number })?.status === 429 ||
                    String(
                        (error as { message?: string })?.message || ''
                    ).includes('429')
            }
        ).catch((error: unknown): null => {
            const message = error instanceof Error ? error.message : '';
            if (message === 'cancelled') {
                return null;
            }
            throw error;
        });

        if (!response || isCancelled()) {
            break;
        }

        const page = Array.isArray(response.json) ? response.json : [];
        collected.push(
            ...page
                .map((entry: { id?: string }) => entry?.id)
                .filter((id): id is string => isValidMutualFriendId(id))
        );

        if (page.length < 100) {
            break;
        }
        offset += page.length;
    }

    return collected;
}
