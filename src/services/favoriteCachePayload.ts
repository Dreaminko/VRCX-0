type FavoriteCacheJsonValue =
    | null
    | boolean
    | number
    | string
    | FavoriteCacheJsonValue[]
    | { [key: string]: FavoriteCacheJsonValue };

export type FavoriteCachePayload = {
    [key: string]: FavoriteCacheJsonValue;
};

function isJsonValue(value: unknown): value is FavoriteCacheJsonValue {
    if (
        value === null ||
        typeof value === 'string' ||
        typeof value === 'number' ||
        typeof value === 'boolean'
    ) {
        return true;
    }
    if (Array.isArray(value)) {
        return value.every(isJsonValue);
    }
    return Boolean(
        value &&
        typeof value === 'object' &&
        Object.values(value).every(isJsonValue)
    );
}

function isFavoriteCachePayload(value: unknown): value is FavoriteCachePayload {
    return (
        isJsonValue(value) &&
        typeof value === 'object' &&
        value !== null &&
        !Array.isArray(value)
    );
}

export function favoriteCachePayload(
    value: unknown
): FavoriteCachePayload | null {
    return isFavoriteCachePayload(value) ? value : null;
}

export function normalizeFavoriteCacheEntityId(value: unknown): string {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}
