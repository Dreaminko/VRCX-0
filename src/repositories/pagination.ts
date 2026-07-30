import {
    VRCHAT_API_DEFAULT_PAGE_SIZE,
    VRCHAT_PROFILE_MAX_PAGES
} from './paginationConstants';

export interface PageRequest {
    n: number;
    offset: number;
}

export interface CollectPagesOptions {
    pageSize?: number;
    maxPages?: number;
}

export async function collectPages<T>(
    fetchPage: (page: PageRequest) => Promise<T[]>,
    {
        pageSize = VRCHAT_API_DEFAULT_PAGE_SIZE,
        maxPages = VRCHAT_PROFILE_MAX_PAGES
    }: CollectPagesOptions = {}
): Promise<T[]> {
    const rows: T[] = [];

    for (let page = 0; page < maxPages; page += 1) {
        const nextRows = await fetchPage({
            n: pageSize,
            offset: page * pageSize
        });
        rows.push(...nextRows);

        if (nextRows.length < pageSize) {
            break;
        }
    }

    return rows;
}
