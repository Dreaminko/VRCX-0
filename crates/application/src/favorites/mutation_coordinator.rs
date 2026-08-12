use std::sync::Arc;

use vrcx_0_application_core::{
    FavoriteChangeScope, FavoritesChangedPayload, RemoteMutationGate, RuntimeAuthScope,
    RuntimeAuthScopeSnapshot, RuntimeDiagnostics, RuntimeEventBus, RuntimeOperationStatus,
    RuntimeSyncEngine, WebClient,
};
use vrcx_0_core::FavoriteEntityKind;
use vrcx_0_persistence::social_aggregates::{self, FavoriteLocalInput, FavoriteOutput};
use vrcx_0_persistence::DatabaseService;

use super::bulk_remove::{remove_favorites_selection, FavoriteBulkRemoveDeps};
use super::favorite_transfer::{transfer_favorite_selection, FavoriteTransferDeps};
use super::local_favorites::{
    add_local_favorite_scoped, create_local_favorite_group_scoped,
    delete_local_favorite_group_scoped, remove_local_favorite_scoped,
    rename_local_favorite_group_scoped, LocalFavoriteMutationDeps,
};
use super::remote_favorites::{
    add_remote_favorite, clear_remote_favorite_group, delete_remote_favorite,
    save_remote_favorite_group, FavoriteRemoteMutationDeps,
};
use super::{
    FavoriteBulkRemoveInput, FavoriteBulkRemoveResult, FavoriteImportLocation,
    FavoriteImportOperation, FavoriteImportStatus, FavoriteRemoteAddInput,
    FavoriteRemoteDeleteInput, FavoriteRemoteGroupClearInput, FavoriteRemoteGroupSaveInput,
    FavoriteTransferSelectionInput, FavoriteTransferSelectionResult, LocalFavoriteGroupWrite,
};
use crate::{AuthenticatedMutationContext, Error, Result};

#[derive(Clone)]
pub struct FavoriteMutationCoordinator {
    db: Arc<DatabaseService>,
    web: Arc<WebClient>,
    diagnostics: RuntimeDiagnostics,
    sync: RuntimeSyncEngine,
    event_bus: RuntimeEventBus,
    auth_scope: RuntimeAuthScope,
    remote_mutations: Arc<RemoteMutationGate>,
}

#[derive(Debug)]
pub enum FavoriteLocalMutationError {
    Persistence(vrcx_0_persistence::Error),
    Application(Error),
}

impl From<vrcx_0_persistence::Error> for FavoriteLocalMutationError {
    fn from(error: vrcx_0_persistence::Error) -> Self {
        Self::Persistence(error)
    }
}

impl FavoriteMutationCoordinator {
    pub fn new(
        db: Arc<DatabaseService>,
        web: Arc<WebClient>,
        diagnostics: RuntimeDiagnostics,
        sync: RuntimeSyncEngine,
        event_bus: RuntimeEventBus,
        auth_scope: RuntimeAuthScope,
        remote_mutations: Arc<RemoteMutationGate>,
    ) -> Self {
        Self {
            db,
            web,
            diagnostics,
            sync,
            event_bus,
            auth_scope,
            remote_mutations,
        }
    }

    fn capture(&self, label: &'static str) -> Result<AuthenticatedMutationContext<'_>> {
        AuthenticatedMutationContext::capture(&self.auth_scope, &self.remote_mutations, label)
    }

    fn local_deps(&self, label: &'static str) -> Result<LocalFavoriteMutationDeps<'_>> {
        Ok(LocalFavoriteMutationDeps {
            db: self.db.as_ref(),
            event_bus: &self.event_bus,
            mutation: self.capture(label)?,
        })
    }

    fn remote_deps(&self, label: &'static str) -> Result<FavoriteRemoteMutationDeps<'_>> {
        Ok(FavoriteRemoteMutationDeps {
            db: self.db.as_ref(),
            web: self.web.as_ref(),
            diagnostics: &self.diagnostics,
            sync: &self.sync,
            event_bus: &self.event_bus,
            mutation: self.capture(label)?,
        })
    }

    fn notify_invalidated(
        &self,
        scope: &RuntimeAuthScopeSnapshot,
        kind: FavoriteChangeScope,
        local_changed: bool,
        remote_changed: bool,
    ) {
        if !local_changed && !remote_changed {
            return;
        }
        self.event_bus
            .emit_favorites_changed(FavoritesChangedPayload::invalidated(
                scope,
                kind,
                local_changed,
                remote_changed,
            ));
    }

    pub fn add_local(
        &self,
        kind: FavoriteEntityKind,
        entity_id: String,
        group_name: String,
    ) -> Result<i64> {
        add_local_favorite_scoped(
            &self.local_deps("Local favorite mutation")?,
            kind,
            entity_id,
            group_name,
        )
    }

    pub fn remove_local(
        &self,
        kind: FavoriteEntityKind,
        entity_id: String,
        group_name: String,
    ) -> Result<i64> {
        remove_local_favorite_scoped(
            &self.local_deps("Local favorite mutation")?,
            kind,
            entity_id,
            group_name,
        )
    }

    pub fn create_local_group(
        &self,
        kind: FavoriteEntityKind,
        group_name: String,
    ) -> Result<LocalFavoriteGroupWrite> {
        create_local_favorite_group_scoped(
            &self.local_deps("Local favorite mutation")?,
            kind,
            group_name,
        )
    }

    pub fn rename_local_group(
        &self,
        kind: FavoriteEntityKind,
        group_name: String,
        new_group_name: String,
    ) -> Result<LocalFavoriteGroupWrite> {
        rename_local_favorite_group_scoped(
            &self.local_deps("Local favorite mutation")?,
            kind,
            group_name,
            new_group_name,
        )
    }

    pub fn delete_local_group(
        &self,
        kind: FavoriteEntityKind,
        group_name: String,
    ) -> Result<LocalFavoriteGroupWrite> {
        delete_local_favorite_group_scoped(
            &self.local_deps("Local favorite mutation")?,
            kind,
            group_name,
        )
    }

    pub fn mutate_local(
        &self,
        label: &'static str,
        input: FavoriteLocalInput,
    ) -> std::result::Result<FavoriteOutput, FavoriteLocalMutationError> {
        let dry_run = input.dry_run;
        let mutation = self
            .capture(label)
            .map_err(FavoriteLocalMutationError::Application)?;
        let output = social_aggregates::favorite_local(
            self.db.as_ref(),
            &mutation.scope().current_user_id,
            input,
        )?;
        if !dry_run {
            mutation
                .ensure_current()
                .map_err(FavoriteLocalMutationError::Application)?;
            self.notify_invalidated(mutation.scope(), output.kind.into(), true, false);
        }
        Ok(output)
    }

    pub async fn add_remote(
        &self,
        label: &'static str,
        input: FavoriteRemoteAddInput,
    ) -> Result<vrcx_0_application_core::vrchat_api::VrchatApiResponse> {
        let deps = self.remote_deps(label)?;
        add_remote_favorite(&deps, input).await
    }

    pub async fn delete_remote(
        &self,
        label: &'static str,
        input: FavoriteRemoteDeleteInput,
    ) -> Result<vrcx_0_application_core::vrchat_api::VrchatApiResponse> {
        let deps = self.remote_deps(label)?;
        delete_remote_favorite(&deps, input).await
    }

    pub async fn save_remote_group(
        &self,
        label: &'static str,
        input: FavoriteRemoteGroupSaveInput,
    ) -> Result<vrcx_0_application_core::vrchat_api::VrchatApiResponse> {
        let deps = self.remote_deps(label)?;
        save_remote_favorite_group(&deps, input).await
    }

    pub async fn clear_remote_group(
        &self,
        label: &'static str,
        input: FavoriteRemoteGroupClearInput,
    ) -> Result<vrcx_0_application_core::vrchat_api::VrchatApiResponse> {
        let deps = self.remote_deps(label)?;
        clear_remote_favorite_group(&deps, input).await
    }

    pub async fn transfer_selection(
        &self,
        input: FavoriteTransferSelectionInput,
    ) -> Result<FavoriteTransferSelectionResult> {
        let command = "app__favorites_transfer_selection";
        let item_count = input
            .batches
            .iter()
            .map(|batch| batch.items.len())
            .sum::<usize>();
        let kind = input
            .batches
            .first()
            .map(|first| {
                if input.batches.iter().all(|batch| batch.kind == first.kind) {
                    first.kind.into()
                } else {
                    FavoriteChangeScope::All
                }
            })
            .unwrap_or(FavoriteChangeScope::All);
        self.diagnostics.record_command(
            command,
            RuntimeOperationStatus::Running,
            format!("Transferring {item_count} favorite item(s)."),
        );
        let mutation = self.capture("Favorite transfer")?;
        let event_scope = mutation.scope().clone();
        let result = transfer_favorite_selection(
            &FavoriteTransferDeps {
                db: self.db.as_ref(),
                web: self.web.as_ref(),
                diagnostics: &self.diagnostics,
                sync: &self.sync,
                mutation,
            },
            input,
        )
        .await;
        match &result {
            Ok(output) => {
                self.diagnostics.record_command(
                    command,
                    RuntimeOperationStatus::Ok,
                    format!("succeeded={}, failed={}", output.succeeded, output.failed),
                );
                self.sync.record(
                    "favorite",
                    RuntimeOperationStatus::Ready,
                    format!(
                        "Transferred {} favorite item(s); {} failed.",
                        output.succeeded, output.failed
                    ),
                    0,
                );
                self.notify_invalidated(
                    &event_scope,
                    kind,
                    output.local_changed,
                    output.remote_changed,
                );
            }
            Err(error) => {
                self.diagnostics.record_command(
                    command,
                    RuntimeOperationStatus::Error,
                    error.to_string(),
                );
                self.sync.record_failure("favorite", error.to_string());
            }
        }
        result
    }

    pub async fn remove_selection(
        &self,
        input: FavoriteBulkRemoveInput,
    ) -> Result<FavoriteBulkRemoveResult> {
        let command = "app__favorites_remove_selection";
        let target_count = input.items.len();
        self.diagnostics.record_command(
            command,
            RuntimeOperationStatus::Running,
            format!("Removing {target_count} favorite item(s)."),
        );
        let expected_scope = self.capture("Favorite bulk remove")?.scope().clone();
        let result = remove_favorites_selection(
            &FavoriteBulkRemoveDeps {
                db: self.db.as_ref(),
                web: self.web.as_ref(),
                auth_scope: &self.auth_scope,
                expected_scope: expected_scope.clone(),
                remote_mutation_gate: &self.remote_mutations,
            },
            input,
        )
        .await;
        match &result {
            Ok(output) => {
                self.diagnostics.record_command(
                    command,
                    RuntimeOperationStatus::Ok,
                    format!("succeeded={}, failed={}", output.succeeded, output.failed),
                );
                self.sync.record(
                    "favorite",
                    RuntimeOperationStatus::Ready,
                    format!(
                        "Removed {} favorite item(s); {} failed.",
                        output.succeeded, output.failed
                    ),
                    0,
                );
                self.notify_invalidated(
                    &expected_scope,
                    output.kind.into(),
                    output.local_changed,
                    output.remote_changed,
                );
            }
            Err(error) => {
                self.diagnostics.record_command(
                    command,
                    RuntimeOperationStatus::Error,
                    error.to_string(),
                );
                self.sync.record_failure("favorite", error.to_string());
            }
        }
        result
    }

    pub(crate) fn complete_import(
        &self,
        scope: &RuntimeAuthScopeSnapshot,
        status: &FavoriteImportStatus,
        location: Option<FavoriteImportLocation>,
    ) {
        if status.operation != FavoriteImportOperation::Import || status.succeeded == 0 {
            return;
        }
        self.notify_invalidated(
            scope,
            status.kind.into(),
            location == Some(FavoriteImportLocation::Local),
            location == Some(FavoriteImportLocation::Remote),
        );
    }

    pub fn complete_shared_collection_import(
        &self,
        scope: &RuntimeAuthScopeSnapshot,
        imported: usize,
    ) {
        if imported > 0 {
            self.notify_invalidated(scope, FavoriteChangeScope::World, true, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use serde_json::json;
    use vrcx_0_persistence::{
        favorites,
        social_aggregates::{FavoriteAction, FavoriteLocalInput},
        storage::StorageService,
    };

    use super::*;
    use crate::{
        FavoriteBulkRemoveItem, FavoriteBulkRemoveSource, FavoriteTransferInput,
        FavoriteTransferItem, FavoriteTransferLocation, FavoriteTransferMode,
        FavoriteTransferSource, FavoriteTransferTarget,
    };

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-favorite-mutations-{name}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    struct Harness {
        _dir: TestDir,
        coordinator: FavoriteMutationCoordinator,
        db: Arc<DatabaseService>,
        event_bus: RuntimeEventBus,
    }

    fn harness(name: &str) -> Harness {
        let dir = TestDir::new(name);
        let db = Arc::new(DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap());
        let storage = StorageService::new(&dir.0.join("storage.json")).unwrap();
        let web = Arc::new(
            WebClient::new(
                &storage,
                db.as_ref(),
                "wss://pipeline.vrchat.cloud".into(),
                env!("CARGO_PKG_VERSION"),
            )
            .unwrap(),
        );
        let auth_scope = RuntimeAuthScope::new();
        auth_scope.set("usr_self", "https://api.vrchat.cloud/api/1");
        let event_bus = RuntimeEventBus::new();
        let coordinator = FavoriteMutationCoordinator::new(
            Arc::clone(&db),
            web,
            RuntimeDiagnostics::new(),
            RuntimeSyncEngine::new(),
            event_bus.clone(),
            auth_scope,
            Arc::new(RemoteMutationGate::default()),
        );
        Harness {
            _dir: dir,
            coordinator,
            db,
            event_bus,
        }
    }

    fn assert_single_local_invalidation(event_bus: &RuntimeEventBus) {
        let events = event_bus.take_events_for_test();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "favoritesChanged");
        assert_eq!(
            events[0].payload,
            json!({
                "ownerUserId": "usr_self",
                "endpoint": "https://api.vrchat.cloud/api/1",
                "kind": "friend",
                "local": true,
                "remote": false,
                "changes": [],
                "requiresRefresh": true
            })
        );
    }

    #[test]
    fn local_mutation_persists_and_emits_one_exact_delta() {
        let harness = harness("local-delta");

        let affected = harness
            .coordinator
            .add_local(
                FavoriteEntityKind::Friend,
                "usr_friend".into(),
                "Close".into(),
            )
            .unwrap();

        assert_eq!(affected, 1);
        assert_eq!(
            favorites::favorite_list(
                harness.db.as_ref(),
                Some("usr_self"),
                FavoriteEntityKind::Friend,
            )
            .unwrap()
            .len(),
            1
        );
        let events = harness.event_bus.take_events_for_test();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "favoritesChanged");
        assert_eq!(
            events[0].payload,
            json!({
                "ownerUserId": "usr_self",
                "endpoint": "https://api.vrchat.cloud/api/1",
                "kind": "friend",
                "local": true,
                "remote": false,
                "changes": [{
                    "type": "localAdded",
                    "kind": "friend",
                    "entityId": "usr_friend",
                    "groupName": "Close"
                }],
                "requiresRefresh": false
            })
        );
    }

    #[test]
    fn tool_dry_run_does_not_persist_or_emit() {
        let harness = harness("tool-dry-run");

        let output = harness
            .coordinator
            .mutate_local(
                "MCP local favorite mutation",
                FavoriteLocalInput {
                    kind: FavoriteEntityKind::Friend,
                    entity_id: "usr_friend".into(),
                    group: "Close".into(),
                    action: FavoriteAction::Add,
                    dry_run: true,
                },
            )
            .unwrap();

        assert_eq!(output.affected_rows, 0);
        assert!(favorites::favorite_list(
            harness.db.as_ref(),
            Some("usr_self"),
            FavoriteEntityKind::Friend,
        )
        .unwrap()
        .is_empty());
        assert!(harness.event_bus.take_events_for_test().is_empty());
    }

    #[test]
    fn tool_write_persists_and_emits_one_invalidation() {
        let harness = harness("tool-write");

        let output = harness
            .coordinator
            .mutate_local(
                "MCP local favorite mutation",
                FavoriteLocalInput {
                    kind: FavoriteEntityKind::Friend,
                    entity_id: "usr_friend".into(),
                    group: "Close".into(),
                    action: FavoriteAction::Add,
                    dry_run: false,
                },
            )
            .unwrap();

        assert_eq!(output.affected_rows, 1);
        assert_single_local_invalidation(&harness.event_bus);
    }

    #[tokio::test]
    async fn local_transfer_emits_once_with_exact_changed_sides() {
        let harness = harness("local-transfer");
        favorites::favorite_add(
            harness.db.as_ref(),
            Some("usr_self"),
            FavoriteEntityKind::Friend,
            "usr_friend".into(),
            "Source".into(),
        )
        .unwrap();

        let output = harness
            .coordinator
            .transfer_selection(FavoriteTransferSelectionInput {
                batches: vec![FavoriteTransferInput {
                    kind: FavoriteEntityKind::Friend,
                    mode: FavoriteTransferMode::Move,
                    source: FavoriteTransferSource {
                        location: FavoriteTransferLocation::Local,
                        group: "Source".into(),
                    },
                    target: FavoriteTransferTarget {
                        location: FavoriteTransferLocation::Local,
                        group: "Target".into(),
                        favorite_type: String::new(),
                    },
                    items: vec![FavoriteTransferItem {
                        key: "local:Source:usr_friend".into(),
                        entity_id: "usr_friend".into(),
                        entity: None,
                    }],
                }],
            })
            .await
            .unwrap();

        assert_eq!(output.succeeded, 1);
        assert_eq!(output.failed, 0);
        assert!(output.local_changed);
        assert!(!output.remote_changed);
        let rows = favorites::favorite_list(
            harness.db.as_ref(),
            Some("usr_self"),
            FavoriteEntityKind::Friend,
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].group_name, "Target");
        assert_single_local_invalidation(&harness.event_bus);
    }

    #[tokio::test]
    async fn local_bulk_remove_emits_once_with_exact_changed_sides() {
        let harness = harness("local-bulk-remove");
        favorites::favorite_add(
            harness.db.as_ref(),
            Some("usr_self"),
            FavoriteEntityKind::Friend,
            "usr_friend".into(),
            "Close".into(),
        )
        .unwrap();

        let output = harness
            .coordinator
            .remove_selection(FavoriteBulkRemoveInput {
                kind: FavoriteEntityKind::Friend,
                items: vec![FavoriteBulkRemoveItem {
                    key: "local:Close:usr_friend".into(),
                    source: FavoriteBulkRemoveSource::Local,
                    entity_id: "usr_friend".into(),
                    group_name: "Close".into(),
                }],
            })
            .await
            .unwrap();

        assert_eq!(output.succeeded, 1);
        assert_eq!(output.failed, 0);
        assert!(output.local_changed);
        assert!(!output.remote_changed);
        assert_single_local_invalidation(&harness.event_bus);
    }

    #[test]
    fn import_completion_emits_only_for_successful_import_writes() {
        let harness = harness("import-completion");
        let scope = RuntimeAuthScopeSnapshot {
            current_user_id: "usr_self".into(),
            endpoint: "https://api.vrchat.cloud/api/1".into(),
            generation: 1,
            active: true,
        };
        let mut status = FavoriteImportStatus {
            operation: FavoriteImportOperation::Hydrate,
            kind: FavoriteEntityKind::Friend,
            succeeded: 1,
            ..FavoriteImportStatus::default()
        };

        harness
            .coordinator
            .complete_import(&scope, &status, Some(FavoriteImportLocation::Local));
        assert!(harness.event_bus.take_events_for_test().is_empty());

        status.operation = FavoriteImportOperation::Import;
        harness
            .coordinator
            .complete_import(&scope, &status, Some(FavoriteImportLocation::Remote));
        let events = harness.event_bus.take_events_for_test();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "favoritesChanged");
        assert_eq!(events[0].payload["local"], false);
        assert_eq!(events[0].payload["remote"], true);
        assert_eq!(events[0].payload["requiresRefresh"], true);
    }
}
