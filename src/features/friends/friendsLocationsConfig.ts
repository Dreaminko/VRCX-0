export const FRIENDS_LOCATIONS_SEGMENTS = [
    { value: 'online', labelKey: 'view.friends_locations.online' },
    { value: 'favorite', labelKey: 'view.friends_locations.favorite' },
    {
        value: 'same-instance',
        labelKey: 'view.friends_locations.same_instance'
    },
    { value: 'active', labelKey: 'view.friends_locations.active' },
    { value: 'offline', labelKey: 'view.friends_locations.offline' }
];

export function buildFriendsLocationsSegmentOptions(
    counts: Record<string, number>
) {
    return FRIENDS_LOCATIONS_SEGMENTS.map((segment) => ({
        ...segment,
        count: counts[segment.value] ?? 0
    }));
}

export function safeJsonParse<T>(value: unknown, fallback: T): unknown | T {
    if (!value) {
        return fallback;
    }

    try {
        return JSON.parse(String(value));
    } catch {
        return fallback;
    }
}

export function parseConfigArray(value: unknown): string[] {
    const parsed = Array.isArray(value) ? value : safeJsonParse(value, []);
    return Array.isArray(parsed)
        ? parsed.filter(
              (entry): entry is string =>
                  typeof entry === 'string' && entry.length > 0
          )
        : [];
}
