import {
    Clock3Icon,
    Globe2Icon,
    PersonStandingIcon,
    Trash2Icon,
    UserRoundIcon,
    UsersRoundIcon
} from 'lucide-react';
import {
    useCallback,
    useDeferredValue,
    useEffect,
    useMemo,
    useRef,
    useState
} from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import {
    EmptyState,
    LoadingState,
    PageBody,
    PageDescription,
    PageHeader,
    PageScaffold,
    PageTitle,
    PageToolbar,
    PageToolbarRow
} from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarSearch,
    ToolbarSegmented,
    ToolbarViews,
    type ToolbarSegmentOption
} from '@/components/layout/ToolbarControls';
import { getVisibleKnownSizeRows } from '@/lib/knownSizeVirtualRows';
import { formatDateTime } from '@/lib/dateTime';
import { cn } from '@/lib/utils';
import { useScrollViewportMetrics } from '@/lib/useScrollViewportMetrics';
import {
    browseHistoryRepository,
    type BrowseHistoryCursor,
    type BrowseHistoryEntityKind,
    type BrowseHistoryItemOutput
} from '@/repositories/browseHistoryRepository';
import { useModalStore } from '@/state/modalStore';
import { useRuntimeStore } from '@/state/runtimeStore';
import { Button } from '@/ui/shadcn/button';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';

import { BrowseHistoryCard } from './BrowseHistoryCard';
import {
    BROWSE_HISTORY_GRID_GAP,
    buildBrowseHistoryRows
} from './browseHistoryRows';

const PAGE_LIMIT = 120;
const CARD_MIN_WIDTH = 292;
const RETENTION_OPTIONS = [7, 30, 90, 365, 0] as const;
type HistoryFilter = 'all' | BrowseHistoryEntityKind;

function isRetentionOption(value: number) {
    return (
        value === 0 ||
        value === 7 ||
        value === 30 ||
        value === 90 ||
        value === 365
    );
}

export function BrowseHistoryPage() {
    const { t } = useTranslation();
    const ownerUserId = useRuntimeStore((state) => state.auth.currentUserId);
    const confirm = useModalStore((state) => state.confirm);
    const [filter, setFilter] = useState<HistoryFilter>('all');
    const [search, setSearch] = useState('');
    const deferredSearch = useDeferredValue(search.trim());
    const [items, setItems] = useState<BrowseHistoryItemOutput[]>([]);
    const [cursor, setCursor] = useState<BrowseHistoryCursor | null>(null);
    const [loading, setLoading] = useState(true);
    const [loadingMore, setLoadingMore] = useState(false);
    const [loadError, setLoadError] = useState(false);
    const [reloadNonce, setReloadNonce] = useState(0);
    const [retentionDays, setRetentionDays] = useState(30);
    const loadMoreLockedRef = useRef(false);
    const requestVersionRef = useRef(0);
    const { resetScrollTop, viewportMetrics, viewportRef } =
        useScrollViewportMetrics();

    const entityKind = filter === 'all' ? null : filter;
    useEffect(() => {
        if (!ownerUserId) {
            setItems([]);
            setCursor(null);
            setLoading(false);
            return;
        }
        const requestVersion = ++requestVersionRef.current;
        setLoading(true);
        setLoadError(false);
        setItems([]);
        setCursor(null);
        resetScrollTop();

        void browseHistoryRepository
            .query({
                ownerUserId,
                entityKind,
                search: deferredSearch,
                cursor: null,
                limit: PAGE_LIMIT
            })
            .then((page) => {
                if (requestVersion === requestVersionRef.current) {
                    setItems(page.items);
                    setCursor(page.nextCursor);
                }
            })
            .catch(() => {
                if (requestVersion === requestVersionRef.current) {
                    setLoadError(true);
                }
            })
            .finally(() => {
                if (requestVersion === requestVersionRef.current) {
                    setLoading(false);
                }
            });
    }, [
        deferredSearch,
        entityKind,
        ownerUserId,
        reloadNonce,
        resetScrollTop
    ]);

    useEffect(() => {
        void browseHistoryRepository
            .getRetentionDays()
            .then(setRetentionDays)
            .catch(() => undefined);
    }, []);

    const safeWidth = Math.max(0, viewportMetrics.width - 8);
    const columnCount = Math.max(
        1,
        Math.floor(
            (safeWidth + BROWSE_HISTORY_GRID_GAP) /
                (CARD_MIN_WIDTH + BROWSE_HISTORY_GRID_GAP)
        ) || 1
    );
    const positioned = useMemo(
        () => buildBrowseHistoryRows(items, columnCount),
        [columnCount, items]
    );
    const visibleRows = useMemo(
        () =>
            getVisibleKnownSizeRows<(typeof positioned.rows)[number]>({
                rows: positioned.rows,
                scrollTop: viewportMetrics.scrollTop,
                viewportHeight: viewportMetrics.viewportHeight,
                overscan: Math.max(520, viewportMetrics.viewportHeight)
            }),
        [
            positioned.rows,
            viewportMetrics.scrollTop,
            viewportMetrics.viewportHeight
        ]
    );

    const loadMore = useCallback(() => {
        if (
            !ownerUserId ||
            !cursor ||
            loadingMore ||
            loadMoreLockedRef.current
        ) {
            return;
        }
        loadMoreLockedRef.current = true;
        setLoadingMore(true);
        const requestVersion = requestVersionRef.current;
        void browseHistoryRepository
            .query({
                ownerUserId,
                entityKind,
                search: deferredSearch,
                cursor,
                limit: PAGE_LIMIT
            })
            .then((page) => {
                if (requestVersion === requestVersionRef.current) {
                    setItems((current) => [...current, ...page.items]);
                    setCursor(page.nextCursor);
                }
            })
            .catch(() => {
                if (requestVersion === requestVersionRef.current) {
                    setCursor(null);
                    toast.error(t('browse_history.load_error'));
                }
            })
            .finally(() => {
                loadMoreLockedRef.current = false;
                setLoadingMore(false);
            });
    }, [cursor, deferredSearch, entityKind, loadingMore, ownerUserId, t]);

    useEffect(() => {
        const remaining =
            positioned.totalHeight -
            viewportMetrics.scrollTop -
            viewportMetrics.viewportHeight;
        if (!loading && remaining < 700) {
            loadMore();
        }
    }, [
        loadMore,
        loading,
        positioned.totalHeight,
        viewportMetrics.scrollTop,
        viewportMetrics.viewportHeight
    ]);

    const filterOptions = useMemo<ToolbarSegmentOption<HistoryFilter>[]>(
        () => [
            { value: 'all', label: t('browse_history.filter.all') },
            {
                value: 'user',
                label: t('browse_history.filter.user'),
                icon: UserRoundIcon
            },
            {
                value: 'world',
                label: t('browse_history.filter.world'),
                icon: Globe2Icon
            },
            {
                value: 'avatar',
                label: t('browse_history.filter.avatar'),
                icon: PersonStandingIcon
            },
            {
                value: 'group',
                label: t('browse_history.filter.group'),
                icon: UsersRoundIcon
            }
        ],
        [t]
    );

    const removeItem = useCallback(
        (item: BrowseHistoryItemOutput) => {
            if (!ownerUserId) {
                return;
            }
            void browseHistoryRepository
                .delete(ownerUserId, item.entityKind, item.entityId)
                .then(() =>
                    setItems((current) =>
                        current.filter(
                            (candidate) =>
                                candidate.entityKind !== item.entityKind ||
                                candidate.entityId !== item.entityId
                        )
                    )
                )
                .catch(() => toast.error(t('browse_history.remove_failed')));
        },
        [ownerUserId, t]
    );

    const clearHistory = useCallback(async () => {
        if (!ownerUserId || !items.length) {
            return;
        }
        const result = await confirm({
            title: t('browse_history.confirmation.title'),
            description: t('browse_history.confirmation.description'),
            confirmText: t('common.actions.clear'),
            cancelText: t('common.actions.cancel'),
            destructive: true
        });
        if (!result.ok) {
            return;
        }
        try {
            await browseHistoryRepository.clear(ownerUserId, entityKind);
            setItems([]);
            setCursor(null);
        } catch {
            toast.error(t('browse_history.clear_failed'));
        }
    }, [confirm, entityKind, items.length, ownerUserId, t]);

    const changeRetention = useCallback(
        (value: unknown) => {
            const next = Number(value);
            if (!ownerUserId || !isRetentionOption(next)) {
                return;
            }
            const previous = retentionDays;
            setRetentionDays(next);
            void browseHistoryRepository
                .setRetentionDays(next)
                .then(() => {
                    setItems((current) =>
                        next === 0
                            ? current
                            : current.filter(
                                  (item) =>
                                      Date.now() -
                                          new Date(item.lastViewedAt).getTime() <=
                                      next * 86_400_000
                              )
                    );
                })
                .catch(() => {
                    setRetentionDays(previous);
                    toast.error(t('browse_history.retention.update_failed'));
                });
        },
        [ownerUserId, retentionDays, t]
    );

    const dayLabel = useCallback(
        (dayKey: string) => {
            const today = new Date();
            const todayKey = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}-${String(today.getDate()).padStart(2, '0')}`;
            const yesterday = new Date(today);
            yesterday.setDate(today.getDate() - 1);
            const yesterdayKey = `${yesterday.getFullYear()}-${String(yesterday.getMonth() + 1).padStart(2, '0')}-${String(yesterday.getDate()).padStart(2, '0')}`;
            if (dayKey === todayKey) {
                return t('browse_history.date.today');
            }
            if (dayKey === yesterdayKey) {
                return t('browse_history.date.yesterday');
            }
            return formatDateTime(`${dayKey}T12:00:00`, {
                year: 'numeric',
                month: 'long',
                day: 'numeric'
            });
        },
        [t]
    );

    return (
        <PageScaffold>
            <PageToolbar>
                <PageHeader>
                    <PageTitle>{t('browse_history.title')}</PageTitle>
                    <PageDescription>
                        {t('browse_history.description')}
                    </PageDescription>
                </PageHeader>
                <PageToolbarRow>
                    <ToolbarViews className="flex-wrap">
                        <ToolbarSegmented
                            value={filter}
                            onValueChange={setFilter}
                            options={filterOptions}
                        />
                    </ToolbarViews>
                    <ToolbarSearch
                        value={search}
                        onValueChange={setSearch}
                        placeholder={t('browse_history.search_placeholder')}
                    />
                    <ToolbarActions>
                        <Select
                            value={String(retentionDays)}
                            onValueChange={changeRetention}
                        >
                            <SelectTrigger size="sm" aria-label={t('browse_history.retention.label')}>
                                <Clock3Icon className="size-3.5" />
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent align="end">
                                {RETENTION_OPTIONS.map((days) => (
                                    <SelectItem key={days} value={String(days)}>
                                        {days === 0
                                            ? t('browse_history.retention.forever')
                                            : t('browse_history.retention.days', {
                                                  count: days
                                              })}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            disabled={!items.length}
                            onClick={() => void clearHistory()}
                        >
                            <Trash2Icon />
                            {t(
                                filter === 'all'
                                    ? 'browse_history.actions.clear_all'
                                    : 'browse_history.actions.clear_kind'
                            )}
                        </Button>
                    </ToolbarActions>
                </PageToolbarRow>
            </PageToolbar>
            <PageBody>
                {loading ? (
                    <LoadingState label={t('browse_history.loading')} />
                ) : loadError ? (
                    <EmptyState
                        icon={Clock3Icon}
                        title={t('browse_history.load_error')}
                    >
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => setReloadNonce((value) => value + 1)}
                        >
                            {t('common.action.retry')}
                        </Button>
                    </EmptyState>
                ) : !items.length ? (
                    <EmptyState
                        icon={Clock3Icon}
                        title={t('browse_history.empty_title')}
                        description={t('browse_history.empty_description')}
                    />
                ) : (
                    <div
                        ref={viewportRef}
                        className="min-h-0 flex-1 overflow-y-auto pr-1"
                    >
                        <div
                            className="relative"
                            style={{ height: positioned.totalHeight }}
                        >
                            {visibleRows.map((row) => (
                                <div
                                    key={row.key}
                                    className={cn(
                                        'absolute right-0 left-0',
                                        row.kind === 'cards' && 'grid'
                                    )}
                                    style={{
                                        top: row.top,
                                        height: row.height,
                                        ...(row.kind === 'cards'
                                            ? {
                                                  gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
                                                  gap: BROWSE_HISTORY_GRID_GAP
                                              }
                                            : {})
                                    }}
                                >
                                    {row.kind === 'heading' ? (
                                        <h2 className="text-muted-foreground flex h-full items-center px-1 text-xs font-medium">
                                            {dayLabel(row.dayKey)}
                                        </h2>
                                    ) : (
                                        row.items.map(
                                            (item: BrowseHistoryItemOutput) => (
                                                <BrowseHistoryCard
                                                    key={`${item.entityKind}:${item.entityId}`}
                                                    item={item}
                                                    onRemove={removeItem}
                                                />
                                            )
                                        )
                                    )}
                                </div>
                            ))}
                        </div>
                        {loadingMore ? (
                            <p className="text-muted-foreground py-2 text-center text-xs">
                                {t('browse_history.loading')}
                            </p>
                        ) : null}
                    </div>
                )}
            </PageBody>
        </PageScaffold>
    );
}
