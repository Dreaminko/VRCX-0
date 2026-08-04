export function emptyArray(value: unknown): unknown[] {
    return Array.isArray(value) ? value : [];
}

export function dedupeById<T extends { id?: unknown }>(
    items: readonly T[] | null | undefined
): T[] {
    const map = new Map<unknown, T>();
    for (const item of items ?? []) {
        if (item?.id) {
            map.set(item.id, item);
        }
    }
    return Array.from(map.values());
}
