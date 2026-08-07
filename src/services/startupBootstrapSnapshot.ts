let prefetchedSystemCulture: string | null = null;

export function primeStartupBootstrapSystemCulture(value: string): void {
    prefetchedSystemCulture = value;
}

export function getPrefetchedSystemCulture(): string | null {
    return prefetchedSystemCulture;
}
