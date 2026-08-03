import {
    useEffect,
    useMemo,
    useRef,
    useState,
    useSyncExternalStore
} from 'react';

import { commands } from '@/platform/tauri/bindings';
import type { FavoriteEntityDetail } from '@/state/favoriteStoreTypes';
import { useRuntimeStore } from '@/state/runtimeStore';

type FavoriteRemoteDetailKind = 'avatar' | 'world';

type FavoriteRemoteEntityDetail = FavoriteEntityDetail & {
    id: string;
};

type FavoriteRemoteDetailsById = Record<string, FavoriteRemoteEntityDetail>;

interface UseFavoriteRemoteDetailsOptions {
    type: FavoriteRemoteDetailKind;
    favoriteIds?: unknown;
    avatarTags?: unknown;
    enabled?: boolean;
    refreshToken?: number;
}

let remoteDetailsRefreshGeneration = 0;
const remoteDetailsRefreshListeners = new Set<() => void>();

export function bumpFavoriteRemoteDetailsRefresh(): void {
    remoteDetailsRefreshGeneration += 1;
    for (const listener of [...remoteDetailsRefreshListeners]) {
        listener();
    }
}

function subscribeToRemoteDetailsRefresh(listener: () => void) {
    remoteDetailsRefreshListeners.add(listener);
    return () => {
        remoteDetailsRefreshListeners.delete(listener);
    };
}

function getRemoteDetailsRefreshGeneration() {
    return remoteDetailsRefreshGeneration;
}

const inflightHydrations = new Map<
    string,
    Promise<Awaited<ReturnType<typeof commands.appFavoriteDetailsHydrate>>>
>();

function hydrateFavoriteDetails(
    requestKey: string,
    input: Parameters<typeof commands.appFavoriteDetailsHydrate>[0]
) {
    const inflight = inflightHydrations.get(requestKey);
    if (inflight) {
        return inflight;
    }
    const request = commands.appFavoriteDetailsHydrate(input).finally(() => {
        inflightHydrations.delete(requestKey);
    });
    inflightHydrations.set(requestKey, request);
    return request;
}

function normalizeValues(values: unknown): string[] {
    return Array.from(
        new Set(
            (Array.isArray(values) ? values : [])
                .map((value) =>
                    typeof value === 'string'
                        ? value.trim()
                        : String(value ?? '').trim()
                )
                .filter(Boolean)
        )
    );
}

function normalizeEntityId(value: unknown) {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

function normalizeOptionalString(value: unknown): string | undefined {
    if (typeof value !== 'string') {
        return undefined;
    }
    const normalized = value.trim();
    return normalized || undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

interface RemoteDetailsState {
    status: string;
    detail: string;
    data: FavoriteRemoteDetailsById;
    availabilityById: Record<string, string>;
    lastLoadedAt: string | null;
}

function buildInitialState(
    status: string = 'idle',
    detail: string = ''
): RemoteDetailsState {
    return {
        status,
        detail,
        data: {},
        availabilityById: {},
        lastLoadedAt: null
    };
}

function mapAvailabilityById(
    availabilityById: unknown
): Record<string, string> {
    const byId: Record<string, string> = {};
    if (!isRecord(availabilityById)) {
        return byId;
    }
    for (const [key, value] of Object.entries(availabilityById)) {
        const id = normalizeEntityId(key);
        const status = normalizeOptionalString(value);
        if (id && status) {
            byId[id] = status;
        }
    }
    return byId;
}

function normalizeFavoriteEntityDetail(
    value: unknown
): FavoriteRemoteEntityDetail | null {
    if (!isRecord(value)) {
        return null;
    }
    const id = normalizeEntityId(value.id);
    if (!id) {
        return null;
    }
    const detail: FavoriteRemoteEntityDetail = {
        ...value,
        id
    };
    if (Array.isArray(value.tags)) {
        detail.tags = normalizeValues(value.tags);
    } else {
        delete detail.tags;
    }

    const releaseStatus = normalizeOptionalString(value.releaseStatus);
    if (releaseStatus) {
        detail.releaseStatus = releaseStatus;
    } else {
        delete detail.releaseStatus;
    }

    const thumbnailImageUrl = normalizeOptionalString(value.thumbnailImageUrl);
    if (thumbnailImageUrl) {
        detail.thumbnailImageUrl = thumbnailImageUrl;
    } else {
        delete detail.thumbnailImageUrl;
    }

    const imageUrl = normalizeOptionalString(value.imageUrl);
    if (imageUrl) {
        detail.imageUrl = imageUrl;
    } else {
        delete detail.imageUrl;
    }

    return detail;
}

function mapDetailsById(detailsById: unknown): FavoriteRemoteDetailsById {
    const byId: FavoriteRemoteDetailsById = {};
    if (!isRecord(detailsById)) {
        return byId;
    }
    for (const value of Object.values(detailsById)) {
        const detail = normalizeFavoriteEntityDetail(value);
        if (!detail) {
            continue;
        }
        byId[detail.id] = detail;
    }
    return byId;
}

export function useFavoriteRemoteDetails({
    type,
    favoriteIds = [],
    avatarTags = [],
    enabled = true,
    refreshToken = 0
}: UseFavoriteRemoteDetailsOptions) {
    const endpoint = useRuntimeStore((state) => state.auth.currentUserEndpoint);
    const refreshGeneration = useSyncExternalStore(
        subscribeToRemoteDetailsRefresh,
        getRemoteDetailsRefreshGeneration
    );
    const normalizedIds = useMemo(
        () => normalizeValues(favoriteIds),
        [favoriteIds]
    );
    const normalizedTags = useMemo(
        () => normalizeValues(avatarTags),
        [avatarTags]
    );
    const requestKey = [
        type,
        endpoint || '',
        normalizedIds.join('|'),
        normalizedTags.join('|'),
        String(refreshToken),
        String(refreshGeneration)
    ].join('::');
    const hasIds = normalizedIds.length > 0;
    const [state, setState] = useState(() => buildInitialState());
    const requestParamsRef = useRef({
        ids: normalizedIds,
        tags: normalizedTags
    });
    requestParamsRef.current = { ids: normalizedIds, tags: normalizedTags };

    useEffect(() => {
        if (!enabled || !hasIds) {
            setState(buildInitialState('ready'));
            return;
        }

        let active = true;
        setState(
            buildInitialState(
                'running',
                type === 'avatar'
                    ? 'Loading remote avatar details.'
                    : 'Loading remote world details.'
            )
        );
        hydrateFavoriteDetails(requestKey, {
            kind: type,
            favoriteIds: requestParamsRef.current.ids,
            avatarTags: type === 'avatar' ? requestParamsRef.current.tags : []
        })
            .then((output) => {
                if (!active) {
                    return;
                }
                const data = mapDetailsById(output.detailsById);
                setState({
                    status: 'ready',
                    detail:
                        type === 'avatar'
                            ? `Loaded remote avatar details for ${Object.keys(data).length} favorites.`
                            : `Loaded remote world details for ${Object.keys(data).length} favorites.`,
                    data,
                    availabilityById: mapAvailabilityById(
                        output.availabilityById
                    ),
                    lastLoadedAt: output.fetchedAt
                });
            })
            .catch((error: unknown) => {
                if (!active) {
                    return;
                }
                setState({
                    status: 'error',
                    detail:
                        error instanceof Error
                            ? error.message
                            : `Failed to load remote ${type} favorites.`,
                    data: {},
                    availabilityById: {},
                    lastLoadedAt: new Date().toISOString()
                });
            });

        return () => {
            active = false;
        };
    }, [enabled, hasIds, requestKey, type]);

    return state;
}
