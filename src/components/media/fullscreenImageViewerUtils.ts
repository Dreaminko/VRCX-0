import { extractFileId } from '@/shared/utils/fileUtils';
function sanitizeFileName(value?: string | null) {
    return String(value || '')
        .replace(/[<>:"/\\|?*]+/g, '_')
        .trim();
}

function ensureImageExtension(fileName?: string | null) {
    if (!fileName) {
        return '';
    }

    return /\.[0-9A-Za-z]+$/.test(fileName) ? fileName : `${fileName}.png`;
}

function getUrlFileName(url?: string | null) {
    if (!url || String(url).startsWith('data:')) {
        return '';
    }

    try {
        const parsedUrl = new URL(url, window.location.href);
        return decodeURIComponent(parsedUrl.pathname.split('/').pop() || '');
    } catch {
        return String(url).split(/[?#]/)[0].split('/').pop() || '';
    }
}

export function toFullSizeImageUrl(url?: string | null) {
    return String(url || '').replace(
        /\/image\/(file_[^/?#]+)\/(\d+)\/\d+\/?(?=([?#]|$))/,
        '/file/$1/$2/file'
    );
}

function getPathFileName(path?: string | null) {
    return String(path || '')
        .split(/[/\\]/)
        .pop();
}

export function deriveImageFileName({
    fileName,
    url,
    sourcePath
}: {
    fileName?: string | null;
    url?: string | null;
    sourcePath?: string | null;
}) {
    const explicitName = ensureImageExtension(sanitizeFileName(fileName));
    if (explicitName) {
        return explicitName;
    }

    const fileId = extractFileId(url || '');
    if (fileId) {
        return `${fileId}.png`;
    }

    const urlFileName = ensureImageExtension(
        sanitizeFileName(getUrlFileName(url))
    );
    if (urlFileName) {
        return urlFileName;
    }

    const sourceFileName = ensureImageExtension(
        sanitizeFileName(getPathFileName(sourcePath))
    );
    return sourceFileName || 'image.png';
}
