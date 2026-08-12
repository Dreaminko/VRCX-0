import { ChevronRightIcon, FolderIcon, RefreshCwIcon } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/utils';
import type {
    ScreenshotFolderInfo,
    ScreenshotFolderTree,
    ScreenshotLibraryImage,
    ScreenshotLibraryScanStatus
} from '@/platform/tauri/bindings';
import { Badge } from '@/ui/shadcn/badge';
import { Button } from '@/ui/shadcn/button';
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle
} from '@/ui/shadcn/card';
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger
} from '@/ui/shadcn/collapsible';
import { Skeleton } from '@/ui/shadcn/skeleton';

import { useScreenshotGalleryGrid } from '../useScreenshotGalleryGrid';
import { EmptyState } from './ScreenshotMetadataParts';
import {
    ScreenshotThumbnailCard,
    useScreenshotThumbnailTitleMap
} from './ScreenshotThumbnailGrid';

type FolderTreeNodeModel = ScreenshotFolderInfo & {
    children: FolderTreeNodeModel[];
};

function buildFolderTree(folderTree: ScreenshotFolderTree | null) {
    const folders = folderTree?.folders ?? [];
    const rootPath = folderTree?.rootPath || folders[0]?.path || '';
    const nodesByPath = new Map<string, FolderTreeNodeModel>();

    for (const folder of folders) {
        nodesByPath.set(folder.path, {
            ...folder,
            children: []
        });
    }

    if (rootPath && !nodesByPath.has(rootPath)) {
        nodesByPath.set(rootPath, {
            path: rootPath,
            parentPath: null,
            name: rootPath,
            imageCount: 0,
            totalImageCount: 0,
            latestModifiedAt: null,
            children: []
        });
    }

    const root = nodesByPath.get(rootPath) || null;
    for (const node of nodesByPath.values()) {
        if (!node.parentPath || node.path === rootPath) {
            continue;
        }
        const parent = nodesByPath.get(node.parentPath);
        if (parent) {
            parent.children.push(node);
        }
    }

    for (const node of nodesByPath.values()) {
        node.children.sort((left, right) =>
            String(left.name || '').localeCompare(String(right.name || ''))
        );
    }

    return root;
}

function folderContainsSelected(
    node: FolderTreeNodeModel | null,
    selectedFolder: string
): boolean {
    if (!node || !selectedFolder) {
        return false;
    }
    if (node.path === selectedFolder) {
        return true;
    }
    return node.children.some((child) =>
        folderContainsSelected(child, selectedFolder)
    );
}

function FolderTreeNode({
    node,
    selectedFolder,
    onSelectFolder
}: {
    node: FolderTreeNodeModel;
    selectedFolder: string;
    onSelectFolder: (folder: string) => void;
}) {
    const containsSelected = folderContainsSelected(node, selectedFolder);
    const [open, setOpen] = useState(() => containsSelected);
    const selected = node.path === selectedFolder;
    const hasChildren = Boolean(node.children?.length);
    const selectedRowRef = useRef<HTMLButtonElement | null>(null);

    useEffect(() => {
        if (containsSelected) {
            setOpen(true);
        }
    }, [containsSelected]);

    useEffect(() => {
        if (selected) {
            selectedRowRef.current?.scrollIntoView({
                block: 'nearest',
                inline: 'nearest'
            });
        }
    }, [selected]);

    const row = (
        <Button
            ref={selected ? selectedRowRef : undefined}
            type="button"
            variant={selected ? 'secondary' : 'ghost'}
            size="sm"
            className="w-full min-w-0 justify-start transition-none"
            aria-current={selected ? 'location' : undefined}
            onClick={() => onSelectFolder(node.path)}
        >
            {hasChildren ? (
                <ChevronRightIcon
                    data-icon="inline-start"
                    className={cn(
                        'transition-transform motion-reduce:transition-none',
                        open && 'rotate-90'
                    )}
                />
            ) : (
                <span aria-hidden="true" className="size-3.5 shrink-0" />
            )}
            <FolderIcon data-icon="inline-start" />
            <span className="truncate text-left" title={node.name}>
                {node.name}
            </span>
            {node.imageCount > 0 && (
                <span
                    aria-hidden="true"
                    className="text-muted-foreground ml-auto min-w-5 text-right text-xs tabular-nums"
                >
                    {node.imageCount}
                </span>
            )}
        </Button>
    );

    if (!hasChildren) {
        return row;
    }

    return (
        <Collapsible open={open} onOpenChange={setOpen}>
            <CollapsibleTrigger render={row} />
            <CollapsibleContent>
                <div className="mt-1 ml-5 flex flex-col gap-1">
                    {node.children.map((child) => (
                        <FolderTreeNode
                            key={child.path}
                            node={child}
                            selectedFolder={selectedFolder}
                            onSelectFolder={onSelectFolder}
                        />
                    ))}
                </div>
            </CollapsibleContent>
        </Collapsible>
    );
}

function ScreenshotGalleryGrid({
    error,
    initialScrollTop,
    images,
    isLoading,
    selectedFolder,
    onOpen,
    onScrollPositionChange
}: {
    error: string;
    initialScrollTop: number;
    images: ScreenshotLibraryImage[];
    isLoading: boolean;
    selectedFolder: string;
    onOpen: (path: string) => void;
    onScrollPositionChange: (folder: string, scrollTop: number) => void;
}) {
    const { t } = useTranslation();
    const {
        gridColumnCount,
        gridGap,
        gridMinWidth,
        totalHeight,
        viewportRef,
        visibleRows
    } = useScreenshotGalleryGrid({
        initialScrollTop,
        items: images,
        resetKey: selectedFolder
    });
    const visibleItems = useMemo(
        () => visibleRows.flatMap((row) => row.items),
        [visibleRows]
    );
    const titleMap = useScreenshotThumbnailTitleMap(visibleItems);

    if (error) {
        return (
            <EmptyState
                title={t('dialog.screenshot_metadata.gallery_load_failed')}
                description={error}
            />
        );
    }

    if (isLoading) {
        return (
            <EmptyState
                loading
                title={t('dialog.screenshot_metadata.loading_gallery')}
                description={t(
                    'dialog.screenshot_metadata.loading_gallery_description'
                )}
            />
        );
    }

    if (!images.length) {
        return (
            <EmptyState
                title={t('dialog.screenshot_metadata.empty_gallery')}
                description={t(
                    'dialog.screenshot_metadata.empty_gallery_description'
                )}
            />
        );
    }

    return (
        <div
            ref={viewportRef}
            className="min-h-0 flex-1 overflow-auto pr-1"
            onScroll={(event) => {
                if (selectedFolder) {
                    onScrollPositionChange?.(
                        selectedFolder,
                        event.currentTarget.scrollTop
                    );
                }
            }}
        >
            <div className="relative" style={{ height: totalHeight }}>
                {visibleRows.map((row) => (
                    <div
                        key={row.key}
                        className="absolute right-0 left-0 grid"
                        style={{
                            top: row.top,
                            gridTemplateColumns: `repeat(${gridColumnCount}, minmax(${gridMinWidth}px, 1fr))`,
                            gap: gridGap
                        }}
                    >
                        {row.items.map((item: ScreenshotLibraryImage) => (
                            <ScreenshotThumbnailCard
                                key={item.path}
                                item={item}
                                onOpen={onOpen}
                                title={titleMap.get(item.path)}
                            />
                        ))}
                    </div>
                ))}
            </div>
        </div>
    );
}

export function ScreenshotGalleryView({
    folderTree,
    images,
    isImagesLoading,
    isTreeLoading,
    error,
    scanStatus,
    selectedFolder,
    onOpenImage,
    onRefresh,
    onSelectFolder,
    onScrollPositionChange,
    restoreScrollTop
}: {
    folderTree: ScreenshotFolderTree | null;
    images: ScreenshotLibraryImage[];
    isImagesLoading: boolean;
    isTreeLoading: boolean;
    error: string;
    scanStatus: ScreenshotLibraryScanStatus | null;
    selectedFolder: string;
    onOpenImage: (path: string) => void;
    onRefresh: () => void;
    onSelectFolder: (folder: string) => void;
    onScrollPositionChange: (folder: string, scrollTop: number) => void;
    restoreScrollTop: number;
}) {
    const { t } = useTranslation();
    const root = useMemo(() => buildFolderTree(folderTree), [folderTree]);

    return (
        <div className="grid min-h-0 flex-1 grid-cols-1 grid-rows-[minmax(160px,240px)_minmax(0,1fr)] gap-4 overflow-hidden lg:grid-cols-[minmax(200px,260px)_minmax(0,1fr)] lg:grid-rows-none xl:grid-cols-[minmax(220px,280px)_minmax(0,1fr)]">
            <aside className="min-h-0">
                <Card size="sm" className="h-full min-h-0">
                    <CardHeader className="border-b">
                        <CardTitle>
                            {t('dialog.screenshot_metadata.folders')}
                        </CardTitle>
                        <CardDescription className="truncate">
                            {error
                                ? t(
                                      'dialog.screenshot_metadata.gallery_load_failed'
                                  )
                                : scanStatus?.running
                                  ? t('dialog.screenshot_metadata.scanning')
                                  : t('dialog.screenshot_metadata.gallery')}
                        </CardDescription>
                        <CardAction>
                            <Button
                                type="button"
                                variant="ghost"
                                size="icon-sm"
                                aria-label={t('common.actions.refresh')}
                                onClick={onRefresh}
                            >
                                <RefreshCwIcon
                                    data-icon="inline-start"
                                    className={cn(
                                        scanStatus?.running && 'animate-spin'
                                    )}
                                />
                            </Button>
                        </CardAction>
                    </CardHeader>
                    <CardContent className="min-h-0 flex-1 overflow-auto">
                        {isTreeLoading ? (
                            <div className="flex flex-col gap-2">
                                <Skeleton className="h-7 w-full" />
                                <Skeleton className="h-7 w-10/12" />
                                <Skeleton className="h-7 w-8/12" />
                            </div>
                        ) : root ? (
                            <FolderTreeNode
                                node={root}
                                selectedFolder={selectedFolder}
                                onSelectFolder={onSelectFolder}
                            />
                        ) : (
                            <EmptyState
                                title={t(
                                    'dialog.screenshot_metadata.empty_folders'
                                )}
                                description={t(
                                    'dialog.screenshot_metadata.empty_folders_description'
                                )}
                            />
                        )}
                    </CardContent>
                </Card>
            </aside>
            <section className="flex min-h-0 min-w-0 flex-col gap-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="min-w-0">
                        <div className="text-sm font-medium">
                            {t('dialog.screenshot_metadata.gallery')}
                        </div>
                        <div className="text-muted-foreground truncate text-xs">
                            {selectedFolder || folderTree?.rootPath || '—'}
                        </div>
                    </div>
                    <Badge variant="outline">
                        {t('dialog.screenshot_metadata.image_count', {
                            count: images.length
                        })}
                    </Badge>
                </div>
                <ScreenshotGalleryGrid
                    error={error}
                    initialScrollTop={restoreScrollTop}
                    images={images}
                    isLoading={isImagesLoading}
                    selectedFolder={selectedFolder}
                    onOpen={onOpenImage}
                    onScrollPositionChange={onScrollPositionChange}
                />
            </section>
        </div>
    );
}
