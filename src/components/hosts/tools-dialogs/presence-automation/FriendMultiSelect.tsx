import { ChevronsUpDownIcon } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { UserPickerRow } from '@/components/search/UserPickerRow';
import type { FriendRosterById } from '@/domain/friends/friendRosterTypes';
import { Button } from '@/ui/shadcn/button';
import { Checkbox } from '@/ui/shadcn/checkbox';
import { Field, FieldLabel } from '@/ui/shadcn/field';
import { Input } from '@/ui/shadcn/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/shadcn/popover';
import { ScrollArea } from '@/ui/shadcn/scroll-area';

const MAX_VISIBLE_OPTIONS = 100;

type FriendPickerOption = {
    label: string;
    search: string;
    user: FriendRosterById[string] | null;
    value: string;
};

function buildFriendPickerOptions(
    selectedIds: string[],
    orderedFriendIds: string[],
    friendsById: FriendRosterById
): FriendPickerOption[] {
    const seen = new Set<string>();
    const options: FriendPickerOption[] = [];

    for (const friendId of [...selectedIds, ...orderedFriendIds]) {
        if (!friendId || seen.has(friendId)) {
            continue;
        }
        seen.add(friendId);
        const user = friendsById[friendId] ?? null;
        const label = user?.displayName || friendId;
        options.push({
            value: friendId,
            label,
            search: `${label} ${friendId}`.toLowerCase(),
            user
        });
    }

    return options;
}

function formatSelectedFriendLabels(labels: string[], emptyLabel: string) {
    if (!labels.length) {
        return emptyLabel;
    }

    const visibleLabels = labels.slice(0, 2).join(', ');
    if (labels.length <= 2) {
        return visibleLabels;
    }

    return `${visibleLabels} +${labels.length - 2}`;
}

export function FriendMultiSelect({
    disabled,
    friendsById,
    idPrefix,
    onChange,
    orderedFriendIds,
    values
}: {
    disabled?: boolean;
    friendsById: FriendRosterById;
    idPrefix: string;
    onChange: (next: string[]) => void;
    orderedFriendIds: string[];
    values: string[];
}) {
    const { t } = useTranslation();
    const [open, setOpen] = useState(false);
    const [search, setSearch] = useState('');
    const selectedIdSet = useMemo(() => new Set(values), [values]);
    const options = useMemo(
        () => buildFriendPickerOptions(values, orderedFriendIds, friendsById),
        [friendsById, orderedFriendIds, values]
    );
    const normalizedSearch = search.trim().toLowerCase();
    const visibleOptions = options
        .filter(
            (option) =>
                !normalizedSearch || option.search.includes(normalizedSearch)
        )
        .slice(0, MAX_VISIBLE_OPTIONS);
    const selectedLabels = options
        .filter((option) => selectedIdSet.has(option.value))
        .map((option) => option.label);
    const triggerLabel = formatSelectedFriendLabels(
        selectedLabels,
        t('common.affinity.friend')
    );

    function toggleFriend(friendId: string) {
        onChange(
            selectedIdSet.has(friendId)
                ? values.filter((value) => value !== friendId)
                : [...values, friendId]
        );
    }

    return (
        <Popover
            open={open}
            onOpenChange={(nextOpen) => {
                setOpen(nextOpen);
                if (!nextOpen) {
                    setSearch('');
                }
            }}
        >
            <PopoverTrigger
                render={
                    <Button
                        type="button"
                        variant="outline"
                        className="w-full justify-between font-normal"
                        disabled={disabled}
                        aria-label={t('common.affinity.friend')}
                    >
                        <span className="truncate">{triggerLabel}</span>
                        <ChevronsUpDownIcon className="text-muted-foreground size-4" />
                    </Button>
                }
            />
            <PopoverContent align="start" className="w-96 p-2">
                <div className="flex flex-col gap-2">
                    <Input
                        value={search}
                        onChange={(event) => setSearch(event.target.value)}
                        placeholder={t('view.friend_list.search_placeholder')}
                    />
                    <ScrollArea className="h-72 rounded-md border">
                        <div className="flex flex-col gap-0.5 p-1 pr-2">
                            {visibleOptions.map((option) => {
                                const selected = selectedIdSet.has(
                                    option.value
                                );
                                const checkboxId = `${idPrefix}-friend-${option.value}`;
                                return (
                                    <Field
                                        key={option.value}
                                        orientation="horizontal"
                                        className="hover:bg-muted gap-0 rounded-md p-0 transition-colors duration-150 ease-out"
                                    >
                                        <Checkbox
                                            id={checkboxId}
                                            checked={selected}
                                            onCheckedChange={() =>
                                                toggleFriend(option.value)
                                            }
                                            className="ml-2"
                                        />
                                        <FieldLabel
                                            htmlFor={checkboxId}
                                            className="min-w-0 flex-1 cursor-pointer font-normal"
                                        >
                                            <UserPickerRow
                                                option={option}
                                                selected={selected}
                                                multiple
                                                showSelection={false}
                                            />
                                        </FieldLabel>
                                    </Field>
                                );
                            })}
                            {!visibleOptions.length ? (
                                <div className="text-muted-foreground p-3 text-xs">
                                    {t('common.search_no_results')}
                                </div>
                            ) : null}
                        </div>
                    </ScrollArea>
                </div>
            </PopoverContent>
        </Popover>
    );
}
