export function asString(value: unknown, fallback: string = ''): string {
    if (value === null || value === undefined) {
        return fallback;
    }
    return String(value);
}

export function safeJsonParse<T = unknown>(
    value: unknown,
    fallback: T | null = null
): T | null {
    if (value === null || value === undefined || value === '') {
        return fallback;
    }

    try {
        return JSON.parse(String(value)) as T;
    } catch {
        return fallback;
    }
}

export function safeJsonStringify(
    value: unknown,
    fallback: string = 'null'
): string {
    try {
        return JSON.stringify(value);
    } catch {
        return fallback;
    }
}
