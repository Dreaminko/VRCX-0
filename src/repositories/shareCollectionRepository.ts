import {
    commands,
    type ShareCollectionCreateInput,
    type ShareCollectionCreateResult
} from '@/platform/tauri/bindings';

export type { ShareCollectionCreateInput, ShareCollectionCreateResult };

export function createShareCollection(
    input: ShareCollectionCreateInput
): Promise<ShareCollectionCreateResult> {
    return commands.appShareCollectionCreate(input);
}

export function openShareCollectionManage(): Promise<null> {
    return commands.appShareCollectionOpenManage();
}

export default Object.freeze({
    createShareCollection,
    openShareCollectionManage
});
