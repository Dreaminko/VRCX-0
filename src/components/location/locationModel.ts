import { parseLocation, normalizeLocationValue } from '@/shared/utils/location';

export type LocationObjectRecord = Record<string, unknown> & {
    tag?: unknown;
    location?: unknown;
    worldId?: unknown;
    world_id?: unknown;
    instanceId?: unknown;
    instance_id?: unknown;
    id?: unknown;
    isOffline?: unknown;
    isPrivate?: unknown;
    isTraveling?: unknown;
    isRealInstance?: unknown;
    accessTypeName?: unknown;
    instanceName?: unknown;
    region?: unknown;
    regionName?: unknown;
    region_name?: unknown;
    shortName?: unknown;
    launchLocation?: unknown;
    inviteLocation?: unknown;
    instanceLocation?: unknown;
    launchToken?: unknown;
    secureOrShortName?: unknown;
    secureName?: unknown;
    strict?: unknown;
    groupId?: unknown;
    userId?: unknown;
    worldName?: unknown;
    world_name?: unknown;
    groupName?: unknown;
    groupDisplayName?: unknown;
    playerCount?: unknown;
    userCount?: unknown;
    occupants?: unknown;
    n_users?: unknown;
    capacity?: unknown;
    users?: unknown[];
    world?: LocationObjectRecord;
    group?: LocationObjectRecord;
    ref?: LocationObjectRecord;
    $location?: LocationObjectRecord;
    $worldName?: unknown;
};

export type NormalizedLocationObject = LocationObjectRecord &
    ReturnType<typeof parseLocation> & { launchToken?: string };

function recordFromUnknown(value: unknown): LocationObjectRecord {
    return value && typeof value === 'object'
        ? (value as LocationObjectRecord)
        : {};
}

export function normalizeLocationText(value: unknown) {
    return typeof value === 'string'
        ? value.trim()
        : String(value ?? '').trim();
}

export function finiteLocationNumber(value: unknown) {
    if (value === null || typeof value === 'undefined' || value === '') {
        return null;
    }
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
}

export function firstNonNegativeLocationNumber(...values: unknown[]) {
    for (const value of values) {
        const number = finiteLocationNumber(value);
        if (number !== null && number >= 0) {
            return number;
        }
    }
    return null;
}

export function firstFiniteLocationNumber(...values: unknown[]) {
    for (const value of values) {
        const number = finiteLocationNumber(value);
        if (number !== null) {
            return number;
        }
    }
    return null;
}

export function resolveLocationTarget(location: unknown, traveling: unknown) {
    const normalizedLocation = normalizeLocationValue(location);
    if (
        typeof traveling !== 'undefined' &&
        normalizedLocation === 'traveling'
    ) {
        return normalizeLocationValue(traveling);
    }
    return normalizedLocation;
}

export function normalizeLocationObject(
    locationObject: unknown
): NormalizedLocationObject {
    if (typeof locationObject === 'string') {
        return parseLocation(locationObject);
    }
    if (locationObject && typeof locationObject === 'object') {
        const source = recordFromUnknown(locationObject);
        const nestedLocation = recordFromUnknown(source.$location);
        const rawTag = normalizeLocationText(
            source.tag || source.location || nestedLocation.tag
        );
        const rawWorldId = normalizeLocationText(
            source.worldId || source.world_id || nestedLocation.worldId
        );
        const rawInstanceId = normalizeLocationText(
            source.instanceId ||
                source.instance_id ||
                source.id ||
                nestedLocation.instanceId
        );
        const synthesizedTag = rawInstanceId.includes(':')
            ? rawInstanceId
            : rawWorldId && rawInstanceId
              ? `${rawWorldId}:${rawInstanceId}`
              : '';
        const tag = rawTag || synthesizedTag;
        const parsed = parseLocation(tag);
        const instanceId =
            rawInstanceId && !rawInstanceId.includes(':')
                ? rawInstanceId
                : parsed.instanceId;

        return {
            ...parsed,
            ...source,
            tag: tag || parsed.tag,
            isOffline: Boolean(source.isOffline ?? parsed.isOffline),
            isPrivate: Boolean(source.isPrivate ?? parsed.isPrivate),
            isTraveling: Boolean(source.isTraveling ?? parsed.isTraveling),
            isRealInstance: Boolean(
                source.isRealInstance ?? parsed.isRealInstance
            ),
            worldId: rawWorldId || parsed.worldId,
            instanceId,
            accessTypeName:
                normalizeLocationText(source.accessTypeName) ||
                parsed.accessTypeName,
            instanceName:
                normalizeLocationText(source.instanceName) ||
                parsed.instanceName,
            region:
                normalizeLocationText(source.region) ||
                normalizeLocationText(source.regionName) ||
                normalizeLocationText(source.region_name) ||
                parsed.region,
            shortName:
                normalizeLocationText(source.shortName) || parsed.shortName,
            launchToken:
                normalizeLocationText(source.launchToken) ||
                normalizeLocationText(source.secureOrShortName) ||
                normalizeLocationText(source.secureName) ||
                normalizeLocationText(source.shortName) ||
                parsed.shortName,
            strict: Boolean(source.strict ?? parsed.strict),
            groupId: normalizeLocationText(source.groupId) || parsed.groupId,
            userId: normalizeLocationText(source.userId) || parsed.userId
        } as NormalizedLocationObject;
    }
    return parseLocation('');
}

export function locationObjectWorldName(locObj: LocationObjectRecord) {
    return normalizeLocationText(
        locObj?.worldName ||
            locObj?.world_name ||
            locObj?.world?.name ||
            locObj?.ref?.worldName ||
            locObj?.ref?.world?.name ||
            locObj?.$worldName ||
            locObj?.$location?.worldName ||
            locObj?.$location?.world?.name ||
            locObj?.$location?.ref?.worldName ||
            locObj?.$location?.ref?.world?.name
    );
}

export function locationObjectGroupName(locObj: LocationObjectRecord) {
    return normalizeLocationText(
        locObj?.groupName ||
            locObj?.group?.name ||
            locObj?.group?.displayName ||
            locObj?.groupDisplayName ||
            locObj?.ref?.groupName ||
            locObj?.ref?.group?.name ||
            locObj?.ref?.group?.displayName ||
            locObj?.ref?.groupDisplayName ||
            locObj?.$location?.groupName ||
            locObj?.$location?.ref?.groupName ||
            locObj?.$location?.ref?.group?.name ||
            locObj?.$location?.ref?.group?.displayName
    );
}

export function worldDialogTarget(locObj: NormalizedLocationObject) {
    return (
        normalizeLocationText(locObj.worldId) ||
        normalizeLocationText(locObj.tag)
    );
}

export function launchTagForLocationObject(locObj: NormalizedLocationObject) {
    const tag = normalizeLocationText(locObj.tag);
    if (tag) {
        return tag;
    }
    const worldId = normalizeLocationText(locObj.worldId);
    const instanceId = normalizeLocationText(locObj.instanceId);
    return worldId && instanceId ? `${worldId}:${instanceId}` : '';
}

export function isUsableInstanceLocation(
    parsedLocation: ReturnType<typeof parseLocation>
) {
    return Boolean(
        parsedLocation?.isRealInstance &&
        parsedLocation.worldId &&
        parsedLocation.instanceId
    );
}

export function buildInstanceActionTarget(
    target: LocationObjectRecord | null = null
) {
    const source = target || {};
    const baseLocation = normalizeLocationText(
        source.location || source.tag
    );
    const resolvedLaunchLocation =
        normalizeLocationText(source.launchLocation) || baseLocation;
    const resolvedInviteLocation =
        normalizeLocationText(source.inviteLocation) || baseLocation;
    const resolvedInstanceLocation =
        normalizeLocationText(source.instanceLocation) || baseLocation;
    const parsedLaunchLocation = parseLocation(resolvedLaunchLocation);
    const parsedInviteLocation =
        resolvedInviteLocation === resolvedLaunchLocation
            ? parsedLaunchLocation
            : parseLocation(resolvedInviteLocation);
    let parsedInstanceLocation = parsedLaunchLocation;
    if (resolvedInstanceLocation !== resolvedLaunchLocation) {
        parsedInstanceLocation =
            resolvedInstanceLocation === resolvedInviteLocation
                ? parsedInviteLocation
                : parseLocation(resolvedInstanceLocation);
    }
    const resolvedShortName =
        normalizeLocationText(source.shortName) ||
        parsedLaunchLocation.shortName ||
        parsedInviteLocation.shortName ||
        parsedInstanceLocation.shortName ||
        '';

    return {
        location: baseLocation,
        launchLocation: resolvedLaunchLocation,
        inviteLocation: resolvedInviteLocation,
        instanceLocation: resolvedInstanceLocation,
        parsedLaunchLocation,
        parsedInviteLocation,
        parsedInstanceLocation,
        isRealLaunchLocation: isUsableInstanceLocation(parsedLaunchLocation),
        isRealInviteLocation: isUsableInstanceLocation(parsedInviteLocation),
        isRealInstanceLocation: isUsableInstanceLocation(
            parsedInstanceLocation
        ),
        shortName: resolvedShortName,
        launchToken:
            normalizeLocationText(source.launchToken) || resolvedShortName,
        worldName: normalizeLocationText(source.worldName)
    };
}
