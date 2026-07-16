import externalApiRepository from '@/repositories/externalApiRepository';
import { getVrcxBuildBadge, isPreviewBuildLabel } from '@/shared/buildLabel';
import { branches } from '@/shared/constants/settings';
import { MINUTE_MS } from '@/shared/constants/time';

import { normalizeReleaseList, sanitizeBranch } from './release';
import type { NormalizedRelease, UpdateOptions } from './types';

const PREVIEW_BADGE_TIMESTAMP_PATTERN =
    /^Preview\s+(?<year>\d{4})(?<month>\d{2})(?<day>\d{2})-(?<hour>\d{2})(?<minute>\d{2})$/i;
const TOKYO_UTC_OFFSET_MINUTES = 9 * 60;

type PreviewStableReleaseUpdateCheckResult = {
    handled: boolean;
    release: NormalizedRelease | null;
};

type PreviewStableReleaseUpdateMode = {
    enabled: boolean;
    check: (options?: UpdateOptions) => Promise<NormalizedRelease | null>;
};

function parsePreviewBuildTimestampMs() {
    if (!isPreviewBuildLabel()) {
        return null;
    }

    const match = PREVIEW_BADGE_TIMESTAMP_PATTERN.exec(getVrcxBuildBadge());
    if (!match?.groups) {
        return null;
    }

    const year = Number(match.groups.year);
    const month = Number(match.groups.month);
    const day = Number(match.groups.day);
    const hour = Number(match.groups.hour);
    const minute = Number(match.groups.minute);
    if (
        month < 1 ||
        month > 12 ||
        day < 1 ||
        day > 31 ||
        hour > 23 ||
        minute > 59
    ) {
        return null;
    }

    const timestamp = Date.UTC(
        year,
        month - 1,
        day,
        hour,
        minute - TOKYO_UTC_OFFSET_MINUTES
    );
    const tokyoDate = new Date(
        timestamp + TOKYO_UTC_OFFSET_MINUTES * MINUTE_MS
    );
    if (
        tokyoDate.getUTCFullYear() !== year ||
        tokyoDate.getUTCMonth() !== month - 1 ||
        tokyoDate.getUTCDate() !== day ||
        tokyoDate.getUTCHours() !== hour ||
        tokyoDate.getUTCMinutes() !== minute
    ) {
        return null;
    }

    return timestamp;
}

function isPreviewStableReleaseUpdateCheckEnabled() {
    return isPreviewBuildLabel();
}

function isStableReleaseNewerThanPreviewBuild(
    release: NormalizedRelease,
    previewBuildTimestampMs: number
) {
    const publishedAt = Date.parse(release.publishedAt);
    return (
        Number.isFinite(publishedAt) && publishedAt > previewBuildTimestampMs
    );
}

async function checkPreviewStableReleaseUpdate(
    options: UpdateOptions = {}
): Promise<NormalizedRelease | null> {
    const previewBuildTimestampMs = parsePreviewBuildTimestampMs();
    if (previewBuildTimestampMs === null) {
        return null;
    }

    const latestRelease = await fetchLatestBranchRelease('Stable', {
        ...options,
        requireInstallerAsset: false
    });
    if (
        !latestRelease ||
        !isStableReleaseNewerThanPreviewBuild(
            latestRelease,
            previewBuildTimestampMs
        )
    ) {
        return null;
    }

    return latestRelease;
}

export async function handlePreviewStableReleaseUpdateCheck(
    options: UpdateOptions = {}
): Promise<PreviewStableReleaseUpdateCheckResult> {
    if (!isPreviewStableReleaseUpdateCheckEnabled()) {
        return {
            handled: false,
            release: null
        };
    }

    return {
        handled: true,
        release: await checkPreviewStableReleaseUpdate(options)
    };
}

export function getPreviewStableReleaseUpdateMode(): PreviewStableReleaseUpdateMode {
    return {
        enabled: isPreviewStableReleaseUpdateCheckEnabled(),
        check: checkPreviewStableReleaseUpdate
    };
}

export async function fetchBranchReleases(
    branch: unknown,
    options: UpdateOptions = {}
): Promise<NormalizedRelease[]> {
    const normalizedBranch = sanitizeBranch(branch);
    const response = await externalApiRepository.fetchGithubReleases({
        url: branches[normalizedBranch].urlReleases,
        headers: {
            Accept: 'application/vnd.github+json'
        }
    });
    if (response.status && response.status !== 200) {
        throw new Error(`GitHub release request failed (${response.status}).`);
    }

    const data =
        typeof response.data === 'string'
            ? JSON.parse(response.data)
            : response.data;
    if (data?.message) {
        throw new Error(data.message);
    }

    return normalizeReleaseList(normalizedBranch, data, options);
}

export async function fetchLatestBranchRelease(
    branch: unknown,
    options: UpdateOptions = {}
): Promise<NormalizedRelease | null> {
    const releases = await fetchBranchReleases(branch, options);
    return releases[0] || null;
}
