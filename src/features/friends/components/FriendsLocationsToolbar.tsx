import { useTranslation } from 'react-i18next';

import { PageToolbar, PageToolbarRow } from '@/components/layout/PageScaffold';
import {
    ToolbarActions,
    ToolbarSearch,
    ToolbarSegmented,
    ToolbarViewMenu,
    ToolbarViews,
    type ToolbarSegmentOption
} from '@/components/layout/ToolbarControls';
import { Field, FieldContent, FieldGroup, FieldLabel } from '@/ui/shadcn/field';
import { Switch } from '@/ui/shadcn/switch';
import { ToggleGroup, ToggleGroupItem } from '@/ui/shadcn/toggle-group';

import { FRIENDS_LOCATIONS_DENSITY_OPTIONS } from '../friendsLocationsDensity';

type FriendsLocationsSegmentOption = {
    value: string;
    labelKey: string;
    count: number;
};

type FriendsLocationsToolbarProps = {
    activeSegment: string;
    segmentOptions: FriendsLocationsSegmentOption[];
    searchQuery: string;
    showSameInstanceInOnline: boolean;
    density: string;
    onActiveSegmentChange: (value: string) => void;
    onSearchQueryChange: (value: string) => void;
    onShowSameInstanceInOnlineChange: (value: boolean) => void;
    onDensityChange: (value: string) => void;
};

export function FriendsLocationsToolbar({
    activeSegment,
    segmentOptions,
    searchQuery,
    showSameInstanceInOnline,
    density,
    onActiveSegmentChange,
    onSearchQueryChange,
    onShowSameInstanceInOnlineChange,
    onDensityChange
}: FriendsLocationsToolbarProps) {
    const { t } = useTranslation();
    const options: ToolbarSegmentOption<string>[] = segmentOptions.map(
        (segment) => ({
            value: segment.value,
            label: t(segment.labelKey),
            count: segment.count
        })
    );

    return (
        <PageToolbar>
            <PageToolbarRow>
                <ToolbarViews>
                    <ToolbarSegmented
                        value={activeSegment}
                        onValueChange={onActiveSegmentChange}
                        options={options}
                    />
                </ToolbarViews>

                <ToolbarSearch
                    value={searchQuery}
                    onValueChange={onSearchQueryChange}
                    placeholder={t('view.friends_locations.search_placeholder')}
                />

                <ToolbarActions>
                    <ToolbarViewMenu contentClassName="p-3">
                        <FieldGroup
                            onClick={(event) => event.stopPropagation()}
                        >
                            <Field orientation="horizontal">
                                <FieldContent>
                                    <FieldLabel htmlFor="friends-locations-same-instance">
                                        {t(
                                            'view.friends_locations.show_same_instance_in_online'
                                        )}
                                    </FieldLabel>
                                </FieldContent>
                                <Switch
                                    id="friends-locations-same-instance"
                                    checked={showSameInstanceInOnline}
                                    onCheckedChange={
                                        onShowSameInstanceInOnlineChange
                                    }
                                />
                            </Field>
                            <Field>
                                <FieldContent>
                                    <FieldLabel>
                                        {t('view.friends_locations.density')}
                                    </FieldLabel>
                                </FieldContent>
                                <ToggleGroup
                                    variant="outline"
                                    size="sm"
                                    spacing={1}
                                    value={density ? [density] : []}
                                    onValueChange={(nextValue) => {
                                        if (nextValue[0]) {
                                            onDensityChange(nextValue[0]);
                                        }
                                    }}
                                    className="grid w-full grid-cols-3"
                                >
                                    {FRIENDS_LOCATIONS_DENSITY_OPTIONS.map(
                                        (option) => (
                                            <ToggleGroupItem
                                                key={option.value}
                                                value={option.value}
                                                aria-label={t(option.labelKey)}
                                                className="w-full min-w-0 justify-center px-2"
                                            >
                                                <span className="truncate">
                                                    {t(option.labelKey)}
                                                </span>
                                            </ToggleGroupItem>
                                        )
                                    )}
                                </ToggleGroup>
                            </Field>
                        </FieldGroup>
                    </ToolbarViewMenu>
                </ToolbarActions>
            </PageToolbarRow>
        </PageToolbar>
    );
}
