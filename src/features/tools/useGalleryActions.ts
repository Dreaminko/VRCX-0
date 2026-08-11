import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import mediaRepository from '@/repositories/mediaRepository';
import currentUserProfileService from '@/services/currentUserProfileService';
import {
    readFileAsBase64,
    withUploadTimeout
} from '@/shared/utils/imageUpload';
import { normalizeVrchatEndpointDomain } from '@/shared/vrchatEndpoint';
import { useModalStore } from '@/state/modalStore';
import { useRuntimeStore } from '@/state/runtimeStore';

import { FILE_TABS, UPLOAD_ASPECT_RATIOS } from './galleryConstants';
import type {
    GalleryActionDeps,
    GalleryAuthTarget,
    GalleryControllerDeps
} from './galleryTypes';
import {
    parseEmojiUploadSettings,
    validateImageFile
} from './inventoryHelpers';
import { createGalleryAssetActions } from './useGalleryAssetActions';
import { useGalleryInventoryActions } from './useGalleryInventoryActions';

function buildProfilePicOverride(endpoint: unknown, fileId: unknown) {
    if (!fileId) {
        return '';
    }
    const base = normalizeVrchatEndpointDomain(endpoint);
    return `${base}/file/${fileId}/1`;
}

function getLocalTimestampString() {
    const date = new Date();
    date.setMinutes(date.getMinutes() - date.getTimezoneOffset());
    return date.toISOString().slice(0, 19);
}

function getRuntimeAuthTarget() {
    const runtimeAuth = useRuntimeStore.getState().auth;
    return {
        userId: runtimeAuth.currentUserId || '',
        endpoint: runtimeAuth.currentUserEndpoint || ''
    };
}

function isRuntimeAuthTarget(authTarget: GalleryAuthTarget) {
    const runtimeAuth = getRuntimeAuthTarget();
    return (
        runtimeAuth.userId === authTarget.userId &&
        runtimeAuth.endpoint === authTarget.endpoint
    );
}

export function useGalleryActions(deps: GalleryControllerDeps) {
    const { t } = useTranslation();
    const confirm = useModalStore((state) => state.confirm);
    const prompt = useModalStore((state) => state.prompt);
    const actionDeps = {
        ...deps,
        FILE_TABS,
        UPLOAD_ASPECT_RATIOS,
        buildProfilePicOverride,
        confirm,
        currentUserProfileService,
        getLocalTimestampString,
        isRuntimeAuthTarget,
        mediaRepository,
        parseEmojiUploadSettings,
        prompt,
        readFileAsBase64,
        t,
        toast,
        useRuntimeStore,
        validateImageFile,
        withUploadTimeout
    } satisfies GalleryActionDeps;
    const assetActions = createGalleryAssetActions(actionDeps);
    const inventoryActions = useGalleryInventoryActions({
        ...actionDeps,
        ...assetActions
    });
    return {
        ...assetActions,
        ...inventoryActions
    };
}
