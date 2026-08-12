import { useEffect, useMemo, useRef, useState } from 'react';

import { commands } from '@/platform/tauri/bindings';
import { useFavoriteRevisionStore } from '@/state/favoriteRevisionStore';
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
    requestedIds?: unknown;
    avatarTags?: unknown;
    cacheKey?: string;
    enabled?: boolean;
    refreshToken?: number;
}

function favoriteRemoteDetailsLoadingDetail(
    type: FavoriteRemoteDetailKind
): string {
    return type === 'avatar'
        ? 'Loading remote avatar details.'
        : 'Loading remote world details.';
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
    requestKey: string;
    status: string;
    detail: string;
    data: FavoriteRemoteDetailsById;
    availabilityById: Record<string, string>;
    lastLoadedAt: string | null;
}

function buildInitialState(
    requestKey: string = '',
    status: string = 'idle',
    detail: string = ''
): RemoteDetailsState {
    return {
        requestKey,
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
    requestedIds = favoriteIds,
    avatarTags = [],
    cacheKey = '',
    enabled = true,
    refreshToken = 0
}: UseFavoriteRemoteDetailsOptions) {
    const currentUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const endpoint = useRuntimeStore((state) => state.auth.currentUserEndpoint);
    const remoteDetailsRevision = useFavoriteRevisionStore(
        (state) => state.remoteDetailsRevisionByKind[type]
    );
    const normalizedIds = useMemo(
        () => normalizeValues(favoriteIds),
        [favoriteIds]
    );
    const normalizedRequestedIds = useMemo(
        () => normalizeValues(requestedIds),
        [requestedIds]
    );
    const normalizedTags = useMemo(
        () => normalizeValues(avatarTags),
        [avatarTags]
    );
    const requestKey = [
        type,
        currentUserId || '',
        endpoint || '',
        normalizedIds.join('|'),
        normalizedRequestedIds.join('|'),
        normalizedTags.join('|'),
        cacheKey,
        String(refreshToken),
        String(remoteDetailsRevision)
    ].join('::');
    const hasIds =
        normalizedIds.length > 0 && normalizedRequestedIds.length > 0;
    const refreshKey = [
        cacheKey,
        String(refreshToken),
        String(remoteDetailsRevision)
    ].join('::');
    const [state, setState] = useState(() => buildInitialState());
    const requestParamsRef = useRef({
        ids: normalizedIds,
        requestedIds: normalizedRequestedIds,
        refreshKey,
        tags: normalizedTags
    });
    requestParamsRef.current = {
        ids: normalizedIds,
        requestedIds: normalizedRequestedIds,
        refreshKey,
        tags: normalizedTags
    };

    useEffect(() => {
        if (!enabled || !hasIds) {
            setState(buildInitialState(requestKey, 'ready'));
            return;
        }

        let active = true;
        setState(
            buildInitialState(
                requestKey,
                'running',
                favoriteRemoteDetailsLoadingDetail(type)
            )
        );
        hydrateFavoriteDetails(requestKey, {
            kind: type,
            favoriteIds: requestParamsRef.current.ids,
            requestedIds: requestParamsRef.current.requestedIds,
            avatarTags: type === 'avatar' ? requestParamsRef.current.tags : [],
            refreshKey: requestParamsRef.current.refreshKey
        })
            .then((output) => {
                if (!active) {
                    return;
                }
                const data = mapDetailsById(output.detailsById);
                setState({
                    requestKey,
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
                    requestKey,
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

    if (state.requestKey === requestKey) {
        return state;
    }
    return buildInitialState(
        requestKey,
        enabled && hasIds ? 'running' : 'ready',
        enabled && hasIds ? favoriteRemoteDetailsLoadingDetail(type) : ''
    );
}
