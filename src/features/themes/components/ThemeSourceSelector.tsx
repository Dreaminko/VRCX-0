import { useTranslation } from 'react-i18next';

import { Badge } from '@/ui/shadcn/badge';
import { Button } from '@/ui/shadcn/button';

import { THEME_MODE_OPTIONS, themeModeLabel } from '../themeHelpers';
import type { useThemesController } from '../useThemesController';
import { ThemeSourceButton } from './ThemesPageParts';

type ThemeSourceSelectorProps = Pick<
    ReturnType<typeof useThemesController>,
    | 'customCssBadge'
    | 'visibleSource'
    | 'selectBuiltInSource'
    | 'selectBackgroundSource'
    | 'selectCommunitySource'
    | 'themeMode'
    | 'updateThemeMode'
>;

export function ThemeSourceSelector({
    customCssBadge,
    visibleSource,
    selectBuiltInSource,
    selectBackgroundSource,
    selectCommunitySource,
    themeMode,
    updateThemeMode
}: ThemeSourceSelectorProps) {
    const { t } = useTranslation();

    return (
        <div className="border-border/70 bg-card/70 flex min-w-0 flex-col gap-3 rounded-lg border px-3 py-2.5">
            <div className="flex min-w-0 flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
                <div className="grid min-w-0 gap-1">
                    <div className="text-sm font-medium">
                        {t('view.themes.summary.header')}
                    </div>
                    {customCssBadge ? (
                        <div>
                            <Badge
                                variant="secondary"
                                className="h-5 rounded-md px-1.5 text-xs font-normal"
                            >
                                {customCssBadge}
                            </Badge>
                        </div>
                    ) : null}
                </div>
                <div className="bg-muted/30 flex min-w-0 flex-wrap gap-1 rounded-lg p-1">
                    <ThemeSourceButton
                        active={visibleSource === 'built-in'}
                        onClick={selectBuiltInSource}
                    >
                        {t('view.themes.source.built_in')}
                    </ThemeSourceButton>
                    <ThemeSourceButton
                        active={visibleSource === 'background'}
                        onClick={selectBackgroundSource}
                    >
                        {t('view.themes.source.background')}
                    </ThemeSourceButton>
                    <ThemeSourceButton
                        active={visibleSource === 'community'}
                        onClick={selectCommunitySource}
                    >
                        {t('view.themes.source.community')}
                    </ThemeSourceButton>
                </div>
            </div>

            {visibleSource === 'built-in' ? (
                <div className="border-border/70 flex min-w-0 flex-col gap-2 border-t pt-2 sm:flex-row sm:items-center sm:justify-between">
                    <div className="text-muted-foreground text-xs">
                        {t('view.themes.source.built_in_description')}
                    </div>
                    <div className="flex flex-wrap gap-1.5">
                        {THEME_MODE_OPTIONS.map((mode) => (
                            <Button
                                key={mode}
                                type="button"
                                size="sm"
                                variant={
                                    themeMode === mode ? 'default' : 'outline'
                                }
                                className="h-7"
                                onClick={() => updateThemeMode(mode)}
                            >
                                {themeModeLabel(mode, t)}
                            </Button>
                        ))}
                    </div>
                </div>
            ) : null}
        </div>
    );
}
