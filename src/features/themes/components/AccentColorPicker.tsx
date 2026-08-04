import { useTranslation } from 'react-i18next';

import { THEME_COLORS } from '@/shared/constants/themes';
import { Button } from '@/ui/shadcn/button';

import { themeColorLabel } from '../themeHelpers';
import type { useThemesController } from '../useThemesController';

type AccentColorPickerProps = Pick<
    ReturnType<typeof useThemesController>,
    'accentControlled' | 'themeColor' | 'updateThemeColor'
>;

export function AccentColorPicker({
    accentControlled,
    themeColor,
    updateThemeColor
}: AccentColorPickerProps) {
    const { t } = useTranslation();

    return (
        <div className="border-border/70 bg-card/60 flex min-w-0 flex-col gap-2 rounded-lg border px-3 py-2.5">
            <div className="flex min-w-0 flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                <div className="text-sm font-medium">
                    {t('view.themes.accent.header')}
                </div>
                {accentControlled ? (
                    <p className="text-muted-foreground text-xs">
                        {t('view.community_themes.installed.accent_controlled')}
                    </p>
                ) : null}
            </div>
            <div className="flex flex-wrap gap-1.5">
                {THEME_COLORS.map((color) => (
                    <Button
                        key={color.key}
                        type="button"
                        size="sm"
                        variant={
                            themeColor === color.key ? 'default' : 'outline'
                        }
                        className="h-7"
                        disabled={accentControlled}
                        onClick={() => updateThemeColor(color.key)}
                    >
                        <span
                            aria-hidden="true"
                            className="border-foreground/10 size-2.5 shrink-0 rounded-full border"
                            style={{
                                backgroundColor: color.swatch
                            }}
                        />
                        {themeColorLabel(color, t)}
                    </Button>
                ))}
            </div>
        </div>
    );
}
