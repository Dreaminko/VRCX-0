export interface WorldDialogRequestContext {
    endpoint: string;
    openNonce: number;
    worldId: string;
}

export function createWorldDialogRequestContext({
    endpoint,
    openNonce,
    worldId
}: WorldDialogRequestContext): WorldDialogRequestContext {
    return { endpoint, openNonce, worldId };
}

export function isSameWorldDialogRequestContext(
    active: WorldDialogRequestContext,
    candidate: WorldDialogRequestContext
) {
    return (
        active.endpoint === candidate.endpoint &&
        active.openNonce === candidate.openNonce &&
        active.worldId === candidate.worldId
    );
}
