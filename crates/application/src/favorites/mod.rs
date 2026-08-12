mod bulk_remove;
mod cache_policy;
mod favorite_details_hydrate;
mod favorite_import;
mod favorite_transfer;
mod local_favorites;
mod remote_favorites;

pub use vrcx_0_persistence::favorites::FavoriteRow;

pub use bulk_remove::{
    remove_favorites_bulk, remove_favorites_selection, FavoriteBulkRemoveDeps,
    FavoriteBulkRemoveInput, FavoriteBulkRemoveItem, FavoriteBulkRemoveItemResult,
    FavoriteBulkRemoveItemState, FavoriteBulkRemoveResult, FavoriteBulkRemoveSource,
    FAVORITE_BULK_REMOVE_MAX_ITEMS,
};
pub use cache_policy::{
    persist_favorite_cache_snapshot, FavoriteCacheKind, FavoriteCacheSnapshotInput,
};
pub use favorite_details_hydrate::{
    FavoriteDetailsHydrateInput, FavoriteDetailsHydrateKind, FavoriteDetailsHydrateOutput,
    FavoriteDetailsRuntime,
};
pub use favorite_import::{
    FavoriteImportItemResult, FavoriteImportItemState, FavoriteImportKind, FavoriteImportLocation,
    FavoriteImportOperation, FavoriteImportRuntime, FavoriteImportStartInput, FavoriteImportState,
    FavoriteImportStatus, FavoriteImportTarget, FAVORITE_IMPORT_MAX_ITEMS,
};
pub use favorite_transfer::{
    favorite_transfer_plan_for_item, transfer_favorite_selection, transfer_favorites,
    FavoriteTransferDeps, FavoriteTransferInput, FavoriteTransferItem, FavoriteTransferItemResult,
    FavoriteTransferItemStatus, FavoriteTransferLocation, FavoriteTransferMode,
    FavoriteTransferResult, FavoriteTransferSelectionInput, FavoriteTransferSelectionResult,
    FavoriteTransferSource, FavoriteTransferStage, FavoriteTransferTarget,
};
pub(crate) use local_favorites::create_local_favorite_group;
pub use local_favorites::{
    add_local_favorite_scoped, create_local_favorite_group_scoped,
    delete_local_favorite_group_scoped, get_local_favorite_snapshot, list_local_favorites,
    remove_local_favorite_scoped, rename_local_favorite_group_scoped, LocalFavoriteGroupWrite,
    LocalFavoriteMutationDeps, LocalFavoriteSnapshot,
};
pub use remote_favorites::{
    add_remote_favorite, clear_remote_favorite_group, delete_remote_favorite,
    save_remote_favorite_group, FavoriteRemoteAddInput, FavoriteRemoteDeleteInput,
    FavoriteRemoteGroupClearInput, FavoriteRemoteGroupSaveInput, FavoriteRemoteMutationDeps,
};
