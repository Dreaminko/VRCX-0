import { ArrowDownToLineIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarFilterMenu,
    ToolbarIconButton,
    ToolbarRefreshButton,
    ToolbarSearch,
    ToolbarViews
} from '@/components/layout/ToolbarControls';
import { formatDateFilter } from '@/lib/dateTime';
import type { VrchatLogFileOutput } from '@/platform/tauri/bindings';
import { Checkbox } from '@/ui/shadcn/checkbox';
import {
    DropdownMenuCheckboxItem,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuSeparator
} from '@/ui/shadcn/dropdown-menu';
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';

import type { useVrchatLogController } from '../useVrchatLogController';
import { fileLabel, LOG_LEVELS } from '../vrchatLogHelpers';

type VrchatLogController = ReturnType<typeof useVrchatLogController>;
type VrchatLogToolbarProps = Pick<
    VrchatLogController,
    | 'selectedFileName'
    | 'setSelectedFileName'
    | 'files'
    | 'isFilesLoading'
    | 'selectedFile'
    | 'isEntriesLoading'
    | 'refresh'
    | 'followLatest'
    | 'setFollowLatest'
    | 'searchQuery'
    | 'setSearchQuery'
    | 'levels'
    | 'toggleLevel'
    | 'categoryOptions'
    | 'selectedCategories'
    | 'setSelectedCategories'
    | 'toggleCategory'
>;

export function VrchatLogToolbar({
    selectedFileName,
    setSelectedFileName,
    files,
    isFilesLoading,
    selectedFile,
    isEntriesLoading,
    refresh,
    followLatest,
    setFollowLatest,
    searchQuery,
    setSearchQuery,
    levels,
    toggleLevel,
    categoryOptions,
    selectedCategories,
    setSelectedCategories,
    toggleCategory
}: VrchatLogToolbarProps) {
    const { t } = useTranslation();

    return (
        <PageToolbar className="border-b">
            <PageToolbarRow>
                <ToolbarViews className="min-w-0">
                    <Select
                        value={selectedFileName}
                        onValueChange={(value) =>
                            setSelectedFileName(value ?? '')
                        }
                        disabled={isFilesLoading || !files.length}
                        items={files.map((file: VrchatLogFileOutput) => ({
                            value: file.fileName,
                            label: fileLabel(
                                file,
                                t('view.tools.vrchat_log.latest')
                            )
                        }))}
                    >
                        <SelectTrigger className="max-w-140 min-w-72 flex-1">
                            <SelectValue
                                placeholder={t(
                                    'view.tools.vrchat_log.file_placeholder'
                                )}
                            />
                        </SelectTrigger>
                        <SelectContent align="start">
                            <SelectGroup>
                                {files.map((file) => (
                                    <SelectItem
                                        key={file.fileName}
                                        value={file.fileName}
                                    >
                                        {fileLabel(
                                            file,
                                            t('view.tools.vrchat_log.latest')
                                        )}
                                    </SelectItem>
                                ))}
                            </SelectGroup>
                        </SelectContent>
                    </Select>
                    {selectedFile?.modifiedAt ? (
                        <span className="text-muted-foreground shrink-0 text-xs tabular-nums">
                            {formatDateFilter(selectedFile.modifiedAt, 'long')}
                        </span>
                    ) : null}
                </ToolbarViews>

                <ToolbarActions>
                    <ToolbarIconButton
                        icon={ArrowDownToLineIcon}
                        active={followLatest}
                        disabled={!selectedFileName}
                        label={t('view.tools.vrchat_log.follow_latest')}
                        onClick={() => setFollowLatest((value) => !value)}
                    />
                    <ToolbarRefreshButton
                        onRefresh={refresh}
                        loading={isFilesLoading || isEntriesLoading}
                    />
                </ToolbarActions>
            </PageToolbarRow>

            <PageToolbarRow>
                <ToolbarViews>
                    <ToolbarFilterMenu
                        activeCount={selectedCategories.length}
                        contentClassName="w-72"
                    >
                        <DropdownMenuGroup>
                            <DropdownMenuItem
                                disabled={!selectedCategories.length}
                                closeOnClick={false}
                                onClick={(event) => {
                                    event.preventDefault();
                                    setSelectedCategories([]);
                                }}
                            >
                                {t('view.tools.vrchat_log.clear_categories')}
                            </DropdownMenuItem>
                        </DropdownMenuGroup>
                        {categoryOptions.length ? (
                            <>
                                <DropdownMenuSeparator />
                                <DropdownMenuGroup>
                                    {categoryOptions.map((option) => (
                                        <DropdownMenuCheckboxItem
                                            key={option}
                                            checked={selectedCategories.includes(
                                                option
                                            )}
                                            onClick={(event) =>
                                                event.preventDefault()
                                            }
                                            onCheckedChange={(checked) =>
                                                toggleCategory(
                                                    option,
                                                    checked === true
                                                )
                                            }
                                        >
                                            <span className="truncate">
                                                {option}
                                            </span>
                                        </DropdownMenuCheckboxItem>
                                    ))}
                                </DropdownMenuGroup>
                            </>
                        ) : null}
                    </ToolbarFilterMenu>

                    <div className="flex shrink-0 items-center gap-1.5">
                        {LOG_LEVELS.map((level) => (
                            <label
                                key={level}
                                className="border-border bg-background text-foreground flex h-8 items-center gap-2 rounded-lg border px-2.5 text-sm"
                            >
                                <Checkbox
                                    checked={levels.includes(level)}
                                    onCheckedChange={(checked) =>
                                        toggleLevel(level, checked === true)
                                    }
                                />
                                <span>{level}</span>
                            </label>
                        ))}
                    </div>
                </ToolbarViews>

                <ToolbarSearch
                    value={searchQuery}
                    onValueChange={setSearchQuery}
                    placeholder={t('view.tools.vrchat_log.search_placeholder')}
                />
            </PageToolbarRow>
        </PageToolbar>
    );
}
