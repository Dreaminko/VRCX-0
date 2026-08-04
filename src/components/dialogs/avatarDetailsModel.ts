import type { AvatarStyleRecord } from '@/repositories/avatarProfileRepository';
import { normalizeString } from '@/shared/utils/string';

export function normalizeTagName(value: unknown, prefix: string) {
    const normalized = String(value || '')
        .trim()
        .toLowerCase()
        .replace(new RegExp(`^${prefix}`), '');
    return normalized ? `${prefix}${normalized}` : '';
}

export function authorTagsFromCsv(value: unknown) {
    return Array.from(
        new Set(
            String(value || '')
                .split(',')
                .map((entry) => normalizeTagName(entry, 'author_tag_'))
                .filter(Boolean)
        )
    );
}

export function authorTagsCsv(tags: unknown) {
    return (Array.isArray(tags) ? tags : [])
        .filter(
            (tag): tag is string =>
                typeof tag === 'string' && tag.startsWith('author_tag_')
        )
        .map((tag) => tag.replace(/^author_tag_/, ''))
        .join(',');
}

export function tagsKey(tags: unknown) {
    return (Array.isArray(tags) ? tags : []).slice().sort().join('\n');
}

export function styleName(style: AvatarStyleRecord | null | undefined) {
    return normalizeString(style?.styleName || style?.name || style?.id);
}
