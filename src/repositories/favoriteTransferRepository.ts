import {
    commands,
    type FavoriteTransferSelectionInput,
    type FavoriteTransferSelectionResult
} from '@/platform/tauri/bindings';

export function transferFavoriteSelection(
    input: FavoriteTransferSelectionInput
): Promise<FavoriteTransferSelectionResult> {
    return commands.appFavoritesTransferSelection(input);
}

export default Object.freeze({
    transferFavoriteSelection
});
