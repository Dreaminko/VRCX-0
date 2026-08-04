import { RefreshCwIcon, UploadIcon, XIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/ui/shadcn/button';
import { TabsContent } from '@/ui/shadcn/tabs';

import { GalleryFileCard } from './GalleryFileCard';
import { EmptyState, LoadingState } from './GalleryViewParts';
import { MediaLibraryToolbar } from './MediaLibraryToolbar';
import type {
    FileAssetTab,
    FileTabDefinition
} from '../galleryConstants';
import type { GalleryFileTabState } from '../galleryTypes';

export function GalleryFileTab({
    tab,
    definition,
    fileTab
}: {
    tab: FileAssetTab;
    definition: FileTabDefinition;
    fileTab: GalleryFileTabState;
}) {
    const {
        assets,
        loadingByTab,
        uploadingTab,
        mutatingKey,
        currentUserId,
        profilePicOverride,
        userIcon,
        gridDensityConfig,
        onRefresh,
        onBeginUpload,
        onClearProfileField,
        onPreview,
        onSetProfileField,
        onDeleteFile
    } = fileTab;
    const files = assets[tab];
    const loading = loadingByTab[tab];
    const { t } = useTranslation();

    return (
        <TabsContent
            value={tab}
            className="mt-2 flex min-h-0 flex-1 data-hidden:hidden"
        >
            <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
                <MediaLibraryToolbar
                    actions={
                        <>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => onRefresh(tab)}
                            >
                                <RefreshCwIcon data-icon="inline-start" />
                                {t('dialog.gallery_icons.refresh')}
                            </Button>
                            <Button
                                variant="outline"
                                size="sm"
                                disabled={Boolean(uploadingTab)}
                                onClick={() => onBeginUpload(tab)}
                            >
                                <UploadIcon data-icon="inline-start" />
                                {t('dialog.gallery_icons.upload')}
                            </Button>
                            {tab === 'gallery' ? (
                                <Button
                                    variant="outline"
                                    size="sm"
                                    disabled={
                                        !profilePicOverride ||
                                        Boolean(mutatingKey)
                                    }
                                    onClick={() =>
                                        onClearProfileField(
                                            'profilePicOverride',
                                            ''
                                        )
                                    }
                                >
                                    <XIcon data-icon="inline-start" />
                                    {t('dialog.gallery_icons.clear_banner')}
                                </Button>
                            ) : null}
                            {tab === 'icons' ? (
                                <Button
                                    variant="outline"
                                    size="sm"
                                    disabled={!userIcon || Boolean(mutatingKey)}
                                    onClick={() =>
                                        onClearProfileField('userIcon', '')
                                    }
                                >
                                    <XIcon data-icon="inline-start" />
                                    {t(
                                        'dialog.gallery_icons.clear_profile_icon'
                                    )}
                                </Button>
                            ) : null}
                        </>
                    }
                />
                <div className="min-h-0 flex-1 overflow-y-auto pr-1">
                    {loading ? (
                        <LoadingState />
                    ) : files.length > 0 ? (
                        <div
                            className={`${gridDensityConfig.fileGridClass} p-1`}
                        >
                            {files.map((file) => (
                                <GalleryFileCard
                                    key={file.id}
                                    tab={tab}
                                    definition={definition}
                                    file={file}
                                    profilePicOverride={profilePicOverride}
                                    userIcon={userIcon}
                                    mutatingKey={mutatingKey}
                                    currentUserId={currentUserId}
                                    onPreview={onPreview}
                                    onSetProfileField={onSetProfileField}
                                    onDeleteFile={onDeleteFile}
                                />
                            ))}
                        </div>
                    ) : (
                        <EmptyState
                            title={t('view.tools.dynamic.no_value_loaded', {
                                value: tab
                            })}
                            description={t(
                                'view.tools.dynamic.refresh_this_tab_to_load_value_files',
                                { value: definition.tag }
                            )}
                        />
                    )}
                </div>
            </div>
        </TabsContent>
    );
}
