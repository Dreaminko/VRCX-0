use std::{
    collections::HashSet,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use vrcx_0_application_core::TaskStopToken;
use vrcx_0_core::vrchat_ids::is_user_id;
use vrcx_0_vrchat_client::http_api::ApiJsonResponse;

use crate::{
    Error, Result, RuntimeAuthScope, RuntimeAuthScopeSnapshot, RuntimeEventBus, TaskSupervisor,
};

use super::service::{ban_member, GroupApiDeps};
use super::types::VrchatGroupUserInput;

const GROUP_BAN_IMPORT_MAX_ITEMS: usize = 1_000;
const GROUP_BAN_IMPORT_INTERVAL: Duration = Duration::from_secs(1);
const GROUP_BAN_IMPORT_CANCEL_POLL: Duration = Duration::from_millis(50);

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupBanImportStartInput {
    pub group_id: String,
    #[serde(default)]
    pub user_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum GroupBanImportState {
    #[default]
    Idle,
    Running,
    Cancelling,
    Completed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum GroupBanImportItemState {
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupBanImportItemResult {
    pub user_id: String,
    pub state: GroupBanImportItemState,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupBanImportStatus {
    pub run_id: String,
    pub status: GroupBanImportState,
    pub group_id: String,
    pub total: usize,
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cancel_requested: bool,
    pub items: Vec<GroupBanImportItemResult>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub last_error: Option<String>,
}

pub type GroupBanImportFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

pub trait GroupBanImportActions: Send + Sync {
    fn ban_user<'a>(&'a self, group_id: &'a str, user_id: &'a str) -> GroupBanImportFuture<'a>;
}

pub struct VrchatGroupBanImportActions {
    pub deps: GroupApiDeps,
}

impl GroupBanImportActions for VrchatGroupBanImportActions {
    fn ban_user<'a>(&'a self, group_id: &'a str, user_id: &'a str) -> GroupBanImportFuture<'a> {
        Box::pin(async move {
            let response = ban_member(
                self.deps.clone(),
                VrchatGroupUserInput {
                    group_id: group_id.to_string(),
                    user_id: user_id.to_string(),
                },
            )
            .await?;
            let response = ApiJsonResponse::parse(response.status, &response.data);
            if response.is_failure() {
                return Err(Error::Custom(
                    response.error_message_or("VRChat group request failed"),
                ));
            }
            Ok(())
        })
    }
}

#[derive(Clone)]
pub struct GroupBanImportRuntime {
    inner: Arc<Mutex<GroupBanImportRuntimeInner>>,
    generation: Arc<AtomicU64>,
    actions: Arc<dyn GroupBanImportActions>,
    event_bus: RuntimeEventBus,
    tasks: TaskSupervisor,
    auth_scope: RuntimeAuthScope,
    interval: Duration,
}

#[derive(Default)]
struct GroupBanImportRuntimeInner {
    status: GroupBanImportStatus,
    cancel: Option<Arc<AtomicBool>>,
}

struct PreparedGroupBanImport {
    group_id: String,
    user_ids: Vec<String>,
}

impl GroupBanImportRuntime {
    pub fn new(
        actions: Arc<dyn GroupBanImportActions>,
        event_bus: RuntimeEventBus,
        tasks: TaskSupervisor,
        auth_scope: RuntimeAuthScope,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(GroupBanImportRuntimeInner::default())),
            generation: Arc::new(AtomicU64::new(0)),
            actions,
            event_bus,
            tasks,
            auth_scope,
            interval: GROUP_BAN_IMPORT_INTERVAL,
        }
    }

    #[cfg(test)]
    fn new_with_interval(
        actions: Arc<dyn GroupBanImportActions>,
        event_bus: RuntimeEventBus,
        tasks: TaskSupervisor,
        auth_scope: RuntimeAuthScope,
        interval: Duration,
    ) -> Self {
        Self {
            interval,
            ..Self::new(actions, event_bus, tasks, auth_scope)
        }
    }

    pub fn status(&self) -> GroupBanImportStatus {
        self.lock_inner().status.clone()
    }

    pub fn start(&self, input: GroupBanImportStartInput) -> Result<GroupBanImportStatus> {
        let prepared = prepare_group_ban_import(input)?;
        let scope = self.auth_scope.snapshot();
        require_active_scope(&scope)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let status = {
            let mut inner = self.lock_inner();
            if is_active_state(inner.status.status) {
                return Err(Error::Custom(
                    "Another group ban import is already active.".into(),
                ));
            }
            let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
            let status = GroupBanImportStatus {
                run_id: format!("group-ban-{}-{generation}", Utc::now().timestamp_millis()),
                status: GroupBanImportState::Running,
                group_id: prepared.group_id.clone(),
                total: prepared.user_ids.len(),
                started_at: Some(Utc::now().to_rfc3339()),
                ..Default::default()
            };
            inner.status = status.clone();
            inner.cancel = Some(Arc::clone(&cancel));
            status
        };
        self.emit_status(status.clone());

        let runtime = self.clone();
        let run_id = status.run_id.clone();
        self.tasks.spawn_cancellable(move |stop_token| async move {
            runtime
                .run_job(run_id, prepared, scope, cancel, stop_token)
                .await;
        });
        Ok(status)
    }

    pub fn cancel(&self) -> GroupBanImportStatus {
        let status = {
            let mut inner = self.lock_inner();
            if !is_active_state(inner.status.status) {
                return inner.status.clone();
            }
            if let Some(cancel) = &inner.cancel {
                cancel.store(true, Ordering::Release);
            }
            inner.status.status = GroupBanImportState::Cancelling;
            inner.status.cancel_requested = true;
            inner.status.clone()
        };
        self.emit_status(status.clone());
        status
    }

    async fn run_job(
        &self,
        run_id: String,
        prepared: PreparedGroupBanImport,
        scope: RuntimeAuthScopeSnapshot,
        cancel: Arc<AtomicBool>,
        stop_token: TaskStopToken,
    ) {
        for (index, user_id) in prepared.user_ids.iter().enumerate() {
            if self.is_cancelled(&scope, cancel.as_ref(), &stop_token) {
                self.finish(&run_id, GroupBanImportState::Cancelled);
                return;
            }
            if index > 0
                && wait_for_interval(self.interval, || {
                    self.is_cancelled(&scope, cancel.as_ref(), &stop_token)
                })
                .await
            {
                self.finish(&run_id, GroupBanImportState::Cancelled);
                return;
            }

            let item = match self.actions.ban_user(&prepared.group_id, user_id).await {
                Ok(()) => GroupBanImportItemResult {
                    user_id: user_id.clone(),
                    state: GroupBanImportItemState::Succeeded,
                    message: String::new(),
                },
                Err(error) => GroupBanImportItemResult {
                    user_id: user_id.clone(),
                    state: GroupBanImportItemState::Failed,
                    message: error.to_string(),
                },
            };
            self.apply_item(&run_id, item);
            if self.is_cancelled(&scope, cancel.as_ref(), &stop_token) {
                self.finish(&run_id, GroupBanImportState::Cancelled);
                return;
            }
        }
        self.finish(&run_id, GroupBanImportState::Completed);
    }

    fn is_cancelled(
        &self,
        scope: &RuntimeAuthScopeSnapshot,
        cancel: &AtomicBool,
        stop_token: &TaskStopToken,
    ) -> bool {
        cancel.load(Ordering::Acquire)
            || stop_token.is_stop_requested()
            || ensure_scope_matches(&self.auth_scope.snapshot(), scope).is_err()
    }

    fn apply_item(&self, run_id: &str, item: GroupBanImportItemResult) {
        let status = {
            let mut inner = self.lock_inner();
            if inner.status.run_id != run_id || !is_active_state(inner.status.status) {
                return;
            }
            inner.status.processed += 1;
            match item.state {
                GroupBanImportItemState::Succeeded => inner.status.succeeded += 1,
                GroupBanImportItemState::Failed => {
                    inner.status.failed += 1;
                    inner.status.last_error = Some(item.message.clone());
                }
            }
            inner.status.items.push(item);
            inner.status.clone()
        };
        self.emit_status(status);
    }

    fn finish(&self, run_id: &str, state: GroupBanImportState) {
        let status = {
            let mut inner = self.lock_inner();
            if inner.status.run_id != run_id || !is_active_state(inner.status.status) {
                return;
            }
            inner.status.status = state;
            inner.status.cancel_requested = false;
            inner.status.finished_at = Some(Utc::now().to_rfc3339());
            inner.cancel = None;
            inner.status.clone()
        };
        self.emit_status(status);
    }

    fn emit_status(&self, status: GroupBanImportStatus) {
        self.event_bus.emit(status);
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, GroupBanImportRuntimeInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn prepare_group_ban_import(input: GroupBanImportStartInput) -> Result<PreparedGroupBanImport> {
    let group_id = input.group_id.trim().to_string();
    if group_id.is_empty() || !group_id.starts_with("grp_") {
        return Err(Error::Custom(
            "Group ban import requires a group id.".into(),
        ));
    }
    let mut seen = HashSet::new();
    let user_ids = input
        .user_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| is_user_id(id))
        .filter(|id| seen.insert(id.clone()))
        .collect::<Vec<_>>();
    if user_ids.is_empty() {
        return Err(Error::Custom(
            "Group ban import requires at least one valid user id.".into(),
        ));
    }
    if user_ids.len() > GROUP_BAN_IMPORT_MAX_ITEMS {
        return Err(Error::Custom(format!(
            "Group ban import cannot exceed {GROUP_BAN_IMPORT_MAX_ITEMS} items."
        )));
    }
    Ok(PreparedGroupBanImport { group_id, user_ids })
}

fn require_active_scope(scope: &RuntimeAuthScopeSnapshot) -> Result<()> {
    if scope.active && !scope.current_user_id.trim().is_empty() {
        Ok(())
    } else {
        Err(Error::Custom(
            "Group ban import requires an authenticated session.".into(),
        ))
    }
}

fn ensure_scope_matches(
    current: &RuntimeAuthScopeSnapshot,
    expected: &RuntimeAuthScopeSnapshot,
) -> Result<()> {
    if current.generation_matches(expected) {
        Ok(())
    } else {
        Err(Error::Custom(
            "Group ban import authentication scope changed.".into(),
        ))
    }
}

fn is_active_state(state: GroupBanImportState) -> bool {
    matches!(
        state,
        GroupBanImportState::Running | GroupBanImportState::Cancelling
    )
}

async fn wait_for_interval(interval: Duration, should_cancel: impl Fn() -> bool) -> bool {
    let started_at = tokio::time::Instant::now();
    loop {
        if should_cancel() {
            return true;
        }
        let elapsed = started_at.elapsed();
        if elapsed >= interval {
            return false;
        }
        tokio::time::sleep((interval - elapsed).min(GROUP_BAN_IMPORT_CANCEL_POLL)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUP_ID: &str = "grp_00000000-0000-0000-0000-000000000001";
    const USER_1: &str = "usr_00000000-0000-0000-0000-000000000001";
    const USER_2: &str = "usr_00000000-0000-0000-0000-000000000002";
    const USER_3: &str = "usr_00000000-0000-0000-0000-000000000003";

    #[derive(Default)]
    struct FakeActions {
        attempts: Arc<Mutex<Vec<String>>>,
        fail_user_id: Option<String>,
        gate: Option<Arc<tokio::sync::Notify>>,
    }

    impl GroupBanImportActions for FakeActions {
        fn ban_user<'a>(
            &'a self,
            _group_id: &'a str,
            user_id: &'a str,
        ) -> GroupBanImportFuture<'a> {
            Box::pin(async move {
                if let Some(gate) = &self.gate {
                    gate.notified().await;
                }
                self.attempts.lock().unwrap().push(user_id.to_string());
                if self.fail_user_id.as_deref() == Some(user_id) {
                    Err(Error::Custom("ban failed".into()))
                } else {
                    Ok(())
                }
            })
        }
    }

    fn runtime_with(
        actions: FakeActions,
    ) -> (GroupBanImportRuntime, TaskSupervisor, RuntimeAuthScope) {
        let tasks = TaskSupervisor::new();
        let auth_scope = RuntimeAuthScope::new();
        auth_scope.set("usr_current", "https://api.vrchat.cloud/api/1");
        let runtime = GroupBanImportRuntime::new_with_interval(
            Arc::new(actions),
            RuntimeEventBus::new(),
            tasks.clone(),
            auth_scope.clone(),
            Duration::ZERO,
        );
        (runtime, tasks, auth_scope)
    }

    fn wait_terminal(runtime: &GroupBanImportRuntime) -> GroupBanImportStatus {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while is_active_state(runtime.status().status) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        runtime.status()
    }

    #[test]
    fn prepare_trims_deduplicates_and_rejects_invalid_ids() {
        let prepared = prepare_group_ban_import(GroupBanImportStartInput {
            group_id: format!(" {GROUP_ID} "),
            user_ids: vec![
                format!(" {USER_1} "),
                USER_1.into(),
                "usr_not-a-valid-id".into(),
                USER_2.into(),
            ],
        })
        .unwrap();

        assert_eq!(prepared.group_id, GROUP_ID);
        assert_eq!(prepared.user_ids, vec![USER_1, USER_2]);
    }

    #[test]
    fn prepare_rejects_missing_group_and_empty_id_lists() {
        assert!(prepare_group_ban_import(GroupBanImportStartInput {
            group_id: String::new(),
            user_ids: vec![USER_1.into()],
        })
        .is_err());
        assert!(prepare_group_ban_import(GroupBanImportStartInput {
            group_id: GROUP_ID.into(),
            user_ids: vec!["usr_bad".into()],
        })
        .is_err());
    }

    #[test]
    fn runs_serially_and_a_failed_item_does_not_stop_later_items() {
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let (runtime, tasks, _auth_scope) = runtime_with(FakeActions {
            attempts: Arc::clone(&attempts),
            fail_user_id: Some(USER_2.into()),
            gate: None,
        });

        let running = runtime
            .start(GroupBanImportStartInput {
                group_id: GROUP_ID.into(),
                user_ids: vec![USER_1.into(), USER_2.into(), USER_3.into()],
            })
            .unwrap();
        assert_eq!(running.status, GroupBanImportState::Running);
        assert!(runtime
            .start(GroupBanImportStartInput {
                group_id: GROUP_ID.into(),
                user_ids: vec![USER_1.into()],
            })
            .unwrap_err()
            .to_string()
            .contains("already active"));

        let terminal = wait_terminal(&runtime);
        assert_eq!(terminal.status, GroupBanImportState::Completed);
        assert_eq!(
            attempts.lock().unwrap().as_slice(),
            &[USER_1, USER_2, USER_3]
        );
        assert_eq!(terminal.succeeded, 2);
        assert_eq!(terminal.failed, 1);
        assert_eq!(terminal.items[1].state, GroupBanImportItemState::Failed);
        assert_eq!(terminal.items[1].message, "ban failed");
        assert_eq!(terminal.last_error.as_deref(), Some("ban failed"));
        tasks.stop_all();
    }

    #[test]
    fn cancel_stops_before_the_next_item() {
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let gate = Arc::new(tokio::sync::Notify::new());
        let (runtime, tasks, _auth_scope) = runtime_with(FakeActions {
            attempts: Arc::clone(&attempts),
            fail_user_id: None,
            gate: Some(Arc::clone(&gate)),
        });

        runtime
            .start(GroupBanImportStartInput {
                group_id: GROUP_ID.into(),
                user_ids: vec![USER_1.into(), USER_2.into()],
            })
            .unwrap();
        let cancelling = runtime.cancel();
        assert_eq!(cancelling.status, GroupBanImportState::Cancelling);
        assert!(cancelling.cancel_requested);
        gate.notify_waiters();
        gate.notify_one();

        let terminal = wait_terminal(&runtime);
        assert_eq!(terminal.status, GroupBanImportState::Cancelled);
        assert!(attempts.lock().unwrap().len() <= 1);
        tasks.stop_all();
    }

    #[test]
    fn auth_scope_change_invalidates_an_active_run() {
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let gate = Arc::new(tokio::sync::Notify::new());
        let (runtime, tasks, auth_scope) = runtime_with(FakeActions {
            attempts: Arc::clone(&attempts),
            fail_user_id: None,
            gate: Some(Arc::clone(&gate)),
        });

        runtime
            .start(GroupBanImportStartInput {
                group_id: GROUP_ID.into(),
                user_ids: vec![USER_1.into(), USER_2.into()],
            })
            .unwrap();
        auth_scope.set("usr_next", "https://api.vrchat.cloud/api/1");
        gate.notify_waiters();
        gate.notify_one();

        let terminal = wait_terminal(&runtime);
        assert_eq!(terminal.status, GroupBanImportState::Cancelled);
        assert!(attempts.lock().unwrap().len() <= 1);
        tasks.stop_all();
    }

    #[test]
    fn start_requires_an_authenticated_session() {
        let tasks = TaskSupervisor::new();
        let runtime = GroupBanImportRuntime::new(
            Arc::new(FakeActions::default()),
            RuntimeEventBus::new(),
            tasks.clone(),
            RuntimeAuthScope::new(),
        );

        assert!(runtime
            .start(GroupBanImportStartInput {
                group_id: GROUP_ID.into(),
                user_ids: vec![USER_1.into()],
            })
            .unwrap_err()
            .to_string()
            .contains("authenticated session"));
        tasks.stop_all();
    }
}
