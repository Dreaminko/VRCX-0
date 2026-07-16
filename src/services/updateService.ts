export type {
    InstallableUpdateRelease,
    NormalizedRelease,
    UpdateDownloadProgress,
    UpdateOptions
} from './update-service/types';

export {
    canInstallUpdatesOnPlatform,
    defaultBranchForVersion,
    getUpdaterManifestAssetName,
    getUpdaterTarget,
    hasUpdateForBranch,
    normalizeGitHubRelease,
    normalizeReleaseList,
    sanitizeBranch
} from './update-service/release';
export {
    fetchBranchReleases,
    fetchLatestBranchRelease,
    getPreviewStableReleaseUpdateMode,
    handlePreviewStableReleaseUpdateCheck
} from './update-service/github';
export {
    checkInstallableUpdate,
    isNoPendingUpdateError,
    isPendingUpdateVersionMismatchError
} from './update-service/tauriRequest';
export { downloadUpdate } from './update-service/download';
export {
    discardPendingUpdate,
    downloadAndInstallUpdate,
    installPendingUpdate
} from './update-service/install';
export { formatReleaseDisplayVersion } from '@/shared/utils/releaseVersion';
