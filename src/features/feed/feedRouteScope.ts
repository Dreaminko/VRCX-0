const FEED_USER_QUERY_KEY = 'user';

function normalizeUserIds(userIds: readonly unknown[]): string[] {
    return [
        ...new Set(
            userIds
                .map((userId) => String(userId ?? '').trim())
                .filter(Boolean)
        )
    ];
}

export function readFeedRouteUserIds(searchParams: URLSearchParams): string[] {
    return normalizeUserIds(searchParams.getAll(FEED_USER_QUERY_KEY));
}

export function withFeedRouteUserIds(
    searchParams: URLSearchParams,
    userIds: readonly unknown[]
): URLSearchParams {
    const nextSearchParams = new URLSearchParams(searchParams);
    nextSearchParams.delete(FEED_USER_QUERY_KEY);
    for (const userId of normalizeUserIds(userIds)) {
        nextSearchParams.append(FEED_USER_QUERY_KEY, userId);
    }
    return nextSearchParams;
}

export function buildFeedRoute(userIds: readonly unknown[]): string {
    const searchParams = withFeedRouteUserIds(
        new URLSearchParams({ feedView: 'table' }),
        userIds
    );
    const query = searchParams.toString();
    return query ? `/feed?${query}` : '/feed';
}
