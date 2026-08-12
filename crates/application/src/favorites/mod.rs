mod bulk_remove;
mod cache_policy;
mod favorite_details_hydrate;
mod favorite_import;
mod favorite_transfer;
mod local_favorites;
mod mutation_coordinator;
mod remote_favorites;

pub use vrcx_0_persistence::favorites::FavoriteRow;

pub use bulk_remove::{
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
    FavoriteImportOperation, FavoriteImportRuntime, FavoriteImportRuntimeDeps,
    FavoriteImportStartInput, FavoriteImportState, FavoriteImportStatus, FavoriteImportTarget,
    FAVORITE_IMPORT_MAX_ITEMS,
};
pub use favorite_transfer::{
    favorite_transfer_plan_for_item, FavoriteTransferInput, FavoriteTransferItem,
    FavoriteTransferItemResult, FavoriteTransferItemStatus, FavoriteTransferLocation,
    FavoriteTransferMode, FavoriteTransferResult, FavoriteTransferSelectionInput,
    FavoriteTransferSelectionResult, FavoriteTransferSource, FavoriteTransferStage,
    FavoriteTransferTarget,
};
pub(crate) use local_favorites::create_local_favorite_group;
pub use local_favorites::{
    get_local_favorite_snapshot, list_local_favorites, LocalFavoriteGroupWrite,
    LocalFavoriteSnapshot,
};
pub use mutation_coordinator::{FavoriteLocalMutationError, FavoriteMutationCoordinator};
pub use remote_favorites::{
    FavoriteRemoteAddInput, FavoriteRemoteDeleteInput, FavoriteRemoteGroupClearInput,
    FavoriteRemoteGroupSaveInput,
};
