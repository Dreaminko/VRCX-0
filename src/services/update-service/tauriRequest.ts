import type { TauriUpdateMetadata } from '@/platform/tauri/bindings';
import {
    checkTauriUpdate,
    type TauriUpdateRequest
} from '@/platform/tauri/updater';
import storageRepository from '@/repositories/storageRepository';

import { fetchLatestBranchRelease } from './github';
import { canInstallUpdatesOnPlatform, getUpdaterTarget } from './release';
import type {
    InstallableUpdateRelease,
    NormalizedRelease,
    UpdateOptions
} from './types';

function errorText(error: unknown): string {
    return error instanceof Error ? error.message : String(error || '');
}

export function isNoPendingUpdateError(error: unknown): boolean {
    return errorText(error).includes('no-pending-update');
}

export function isPendingUpdateVersionMismatchError(error: unknown): boolean {
    return errorText(error).includes('pending-update-version-mismatch');
}

async function getUpdaterProxy() {
    const [proxyEnabledRaw, proxyRaw] = await Promise.all([
        storageRepository.getString('VRCX_ProxyEnabled', '').catch(() => ''),
        storageRepository.getString('VRCX_ProxyServer', '').catch(() => '')
    ]);
    const proxy = String(proxyRaw || '').trim();
    const enabledText = String(proxyEnabledRaw || '')
        .trim()
        .toLowerCase();
    const proxyEnabled = enabledText
        ? ['true', '1', 'yes', 'on'].includes(enabledText)
        : proxy !== '';
    return proxyEnabled ? proxy : '';
}

function shouldAllowDowngradesForBranch() {
    return false;
}

export async function buildTauriUpdaterRequest(
    release: NormalizedRelease,
    hostPlatform: string,
    hostArch: string,
    linuxPackageKind: string
): Promise<TauriUpdateRequest> {
    if (!canInstallUpdatesOnPlatform(hostPlatform)) {
        throw new Error(`Updates are not installable on ${hostPlatform}.`);
    }

    const target =
        release?.target ||
        getUpdaterTarget(hostPlatform, hostArch, linuxPackageKind);
    if (!target) {
        throw new Error('No Tauri updater target is available.');
    }
    const manifestUrl = release?.manifestUrl;
    if (!manifestUrl) {
        throw new Error('Selected release has no Tauri updater manifest.');
    }

    const proxy = await getUpdaterProxy();
    return {
        manifestUrl,
        target,
        allowDowngrades: shouldAllowDowngradesForBranch(),
        proxy: proxy || null
    };
}

async function checkTauriUpdateForRelease(
    release: NormalizedRelease,
    options: UpdateOptions = {}
): Promise<TauriUpdateMetadata | null> {
    const request = await buildTauriUpdaterRequest(
        release,
        options.hostPlatform || 'unknown',
        options.hostArch || 'unknown',
        options.linuxPackageKind || 'unknown'
    );
    return checkTauriUpdate(request);
}

export async function checkInstallableUpdate(
    branch: unknown,
    {
        hostPlatform = 'unknown',
        hostArch = 'unknown',
        linuxPackageKind = 'unknown'
    }: UpdateOptions = {}
): Promise<InstallableUpdateRelease | null> {
    if (!canInstallUpdatesOnPlatform(hostPlatform)) {
        return null;
    }

    const release = await fetchLatestBranchRelease(branch, {
        hostArch,
        linuxPackageKind,
        hostPlatform,
        requireInstallerAsset: true
    });
    if (!release) {
        return null;
    }

    const update = await checkTauriUpdateForRelease(release, {
        branch,
        hostArch,
        linuxPackageKind,
        hostPlatform
    });
    if (!update) {
        return null;
    }
    return {
        ...release,
        ...update,
        body: update.body ?? release.body,
        canonicalVersion: release.canonicalVersion,
        displayVersion: release.displayVersion,
        displayName: release.displayName,
        publishedAt: release.publishedAt,
        tagName: release.tagName,
        htmlUrl: release.htmlUrl
    };
}
