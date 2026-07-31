import { commands } from '@/platform/tauri/bindings';
import { normalizeString } from '@/shared/utils/string';

type ExternalHeaders = Record<string, string>;

async function searchAvatarProvider({
    url,
    vrcxId
}: {
    url: string;
    vrcxId: string;
}) {
    return commands.appExternalApiAvatarSearchGet({ url, vrcxId });
}

async function fetchYoutubeVideoMetadata({
    videoId,
    apiKey
}: {
    videoId: unknown;
    apiKey: unknown;
}) {
    const normalizedVideoId = normalizeString(videoId);
    const normalizedApiKey = normalizeString(apiKey);
    return commands.appExternalApiYoutubeVideoMetadataGet({
        videoId: normalizedVideoId,
        apiKey: normalizedApiKey
    });
}

async function fetchVrcStatusJson(path: string) {
    return commands.appExternalApiVrcStatusJsonGet({ path });
}

async function fetchGithubReleases({
    url,
    headers = {}
}: {
    url: string;
    headers?: ExternalHeaders;
}) {
    return commands.appExternalApiGithubReleasesGet({
        url,
        headers
    });
}

async function fetchGithubContributors({
    url,
    headers = {}
}: {
    url: string;
    headers?: ExternalHeaders;
}) {
    return commands.appExternalApiGithubContributorsGet({
        url,
        headers
    });
}

async function fetchImageDataUrl(url: string) {
    return commands.appExternalApiImageDataUrlGet({ url });
}

const externalApiRepository = Object.freeze({
    searchAvatarProvider,
    fetchYoutubeVideoMetadata,
    fetchVrcStatusJson,
    fetchGithubReleases,
    fetchGithubContributors,
    fetchImageDataUrl
});

export {
    fetchGithubContributors,
    fetchGithubReleases,
    fetchImageDataUrl,
    fetchVrcStatusJson,
    fetchYoutubeVideoMetadata,
    searchAvatarProvider
};
export default externalApiRepository;
