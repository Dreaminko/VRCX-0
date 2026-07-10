import { CopyIcon, MoveRightIcon, Trash2Icon, XIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/ui/shadcn/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger
} from '@/ui/shadcn/dropdown-menu';

import type { FavoriteGroup } from '../favoritesTypes';
import { isFavoriteMoveTargetOverCapacity } from '../favoriteTransfer';

type FavoritesSelectionBarProps = {
    selectedCount: number;
    isAllSelected: boolean;
    moveTargets: FavoriteGroup[];
    showCopyButton: boolean;
    actionsDisabled: boolean;
    onSelectAll(): void;
    onClearSelection(): void;
    onCopySelection(): void;
    onMoveSelection(target: FavoriteGroup): void;
    onBulkRemove(): void;
};

function favoriteMoveTargetLabel(target: FavoriteGroup): string {
    if (typeof target.capacity === 'number') {
        return `${target.label} (${target.count ?? 0}/${target.capacity})`;
    }
    if (typeof target.count === 'number') {
        return `${target.label} (${target.count})`;
    }
    return target.label;
}

function FavoritesSelectionBar({
    selectedCount,
    isAllSelected,
    moveTargets,
    showCopyButton,
    actionsDisabled,
    onSelectAll,
    onClearSelection,
    onCopySelection,
    onMoveSelection,
    onBulkRemove
}: FavoritesSelectionBarProps) {
    const { t } = useTranslation();

    if (selectedCount === 0) {
        return null;
    }

    const remoteMoveTargets = moveTargets.filter(
        (target) => target.source === 'remote'
    );
    const localMoveTargets = moveTargets.filter(
        (target) => target.source === 'local'
    );
    const hasMoveTargets = moveTargets.length > 0;
    const showMoveSeparator =
        remoteMoveTargets.length > 0 && localMoveTargets.length > 0;

    return (
        <div className="pointer-events-none absolute inset-x-0 bottom-3 z-20 flex justify-center px-2">
            <div className="bg-popover text-popover-foreground pointer-events-auto flex max-w-full flex-wrap items-center gap-1.5 rounded-full border px-3 py-1.5 text-sm shadow-lg">
                <span className="text-muted-foreground px-1.5 font-medium whitespace-nowrap">
                    {t('view.favorite.selection.count', {
                        count: selectedCount
                    })}
                </span>
                <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={onSelectAll}
                >
                    {isAllSelected
                        ? t('view.favorite.deselect_all')
                        : t('view.favorite.select_all')}
                </Button>
                {showCopyButton ? (
                    <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        disabled={actionsDisabled}
                        onClick={onCopySelection}
                    >
                        <CopyIcon data-icon="inline-start" />
                        {t('common.actions.copy')}
                    </Button>
                ) : null}
                <DropdownMenu>
                    <DropdownMenuTrigger
                        render={
                            <Button
                                type="button"
                                size="sm"
                                variant="ghost"
                                disabled={actionsDisabled || !hasMoveTargets}
                            >
                                <MoveRightIcon data-icon="inline-start" />
                                {t('view.favorite.action.move')}
                            </Button>
                        }
                    />
                    <DropdownMenuContent align="center" className="w-64">
                        <DropdownMenuGroup>
                            <DropdownMenuLabel>
                                {t('view.favorite.action.move_to')}
                            </DropdownMenuLabel>
                            {remoteMoveTargets.map((target) => (
                                <DropdownMenuItem
                                    key={`remote:${target.key}`}
                                    disabled={isFavoriteMoveTargetOverCapacity(
                                        target,
                                        selectedCount
                                    )}
                                    onClick={() => onMoveSelection(target)}
                                >
                                    {favoriteMoveTargetLabel(target)}
                                </DropdownMenuItem>
                            ))}
                            {showMoveSeparator ? (
                                <DropdownMenuSeparator />
                            ) : null}
                            {localMoveTargets.map((target) => (
                                <DropdownMenuItem
                                    key={`local:${target.key}`}
                                    disabled={isFavoriteMoveTargetOverCapacity(
                                        target,
                                        selectedCount
                                    )}
                                    onClick={() => onMoveSelection(target)}
                                >
                                    {favoriteMoveTargetLabel(target)}
                                </DropdownMenuItem>
                            ))}
                        </DropdownMenuGroup>
                    </DropdownMenuContent>
                </DropdownMenu>
                <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    disabled={actionsDisabled}
                    onClick={onBulkRemove}
                >
                    <Trash2Icon data-icon="inline-start" />
                    {t('view.favorite.bulk_unfavorite')}
                </Button>
                <Button
                    type="button"
                    size="icon-xs"
                    variant="ghost"
                    className="rounded-full"
                    aria-label={t('common.actions.clear')}
                    onClick={onClearSelection}
                >
                    <XIcon data-icon="icon" />
                </Button>
            </div>
        </div>
    );
}

export { FavoritesSelectionBar };
