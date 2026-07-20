use std::time::Duration;

pub use vrcx_0_application_core::{FriendProfileBulkLoadStatus, FriendProfileLoadStatusPayload};

use super::state::ActiveRealtimeContext;
use super::*;

const FRIEND_PROFILE_BULK_LOAD_MAX_RETRIES: u32 = 4;
const FRIEND_PROFILE_BULK_LOAD_BASE_DELAY_MS: u64 = 500;
const FRIEND_PROFILE_BULK_LOAD_REQUEST_INTERVAL_MS: u64 = 1_000;

#[derive(Default)]
pub struct FriendProfileBulkLoadState {
    run_id: u64,
    status: FriendProfileBulkLoadStatus,
    owner: Option<ActiveRealtimeContext>,
    total: u32,
    processed: u32,
    loaded: u32,
    failed: u32,
    started_at: String,
    finished_at: Option<String>,
}

fn friend_profile_bulk_load_owner_matches(
    owner: Option<&ActiveRealtimeContext>,
    active: &ActiveRealtimeContext,
) -> bool {
    owner
        .map(|owner| {
            owner.generation == active.generation
                && owner.client_run_id == active.client_run_id
                && owner.session_generation == active.session_generation
                && owner.session == active.session
        })
        .unwrap_or(false)
}

impl FriendProfileBulkLoadState {
    fn payload(&self) -> FriendProfileLoadStatusPayload {
        FriendProfileLoadStatusPayload {
            run_id: self.run_id,
            status: self.status,
            total: self.total,
            processed: self.processed,
            loaded: self.loaded,
            failed: self.failed,
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
        }
    }
}

fn is_active_bulk_load_status(status: FriendProfileBulkLoadStatus) -> bool {
    matches!(
        status,
        FriendProfileBulkLoadStatus::Running | FriendProfileBulkLoadStatus::Cancelling
    )
}

pub(super) fn select_friend_profile_bulk_load_targets(
    friends_by_id: &HashMap<String, FriendRecord>,
) -> Vec<String> {
    let mut ids: Vec<String> = friends_by_id
        .values()
        .filter(|friend| !friend.id.trim().is_empty() && friend_missing_date_joined(friend))
        .map(|friend| friend.id.clone())
        .collect();
    ids.sort();
    ids
}

fn friend_missing_date_joined(friend: &FriendRecord) -> bool {
    match friend.extra.get("date_joined") {
        None => true,
        Some(Value::Null) => true,
        Some(Value::String(value)) => value.trim().is_empty(),
        Some(_) => false,
    }
}

pub(super) fn friend_profile_bulk_load_backoff_delay_ms(attempt: u32) -> u64 {
    FRIEND_PROFILE_BULK_LOAD_BASE_DELAY_MS.saturating_mul(1u64 << attempt.min(16))
}

pub(super) fn friend_profile_bulk_load_initial_progress(
    total_friends: usize,
    pending_friends: usize,
) -> (u32, u32) {
    let total = u32::try_from(total_friends).unwrap_or(u32::MAX);
    let pending = u32::try_from(pending_friends)
        .unwrap_or(u32::MAX)
        .min(total);
    (total, total.saturating_sub(pending))
}

impl RealtimeHostRuntime {
    pub fn start_friend_profile_bulk_load(
        self: &Arc<Self>,
    ) -> Result<FriendProfileLoadStatusPayload> {
        let active = {
            let state = self
                .state
                .lock()
                .map_err(|error| Error::Custom(format!("realtime state lock: {error}")))?;
            state.connection.active_context.clone().ok_or_else(|| {
                Error::Custom(
                    "Friend profile bulk load requires an active realtime session.".into(),
                )
            })?
        };
        let (targets, run_id, spawn_worker, stale_run_id) = {
            let mut bulk = self.friend_profile_bulk_load.lock().map_err(|error| {
                Error::Custom(format!("friend profile bulk load lock: {error}"))
            })?;
            if is_active_bulk_load_status(bulk.status)
                && friend_profile_bulk_load_owner_matches(bulk.owner.as_ref(), &active)
            {
                return Ok(bulk.payload());
            }
            let snapshot = self.friends.snapshot().filter(|snapshot| {
                snapshot.generation == active.generation
                    && snapshot.current_user_id == active.session.user_id
            });
            let Some(snapshot) = snapshot else {
                return Err(Error::Custom(
                    "Friend profile bulk load requires a loaded friend roster.".into(),
                ));
            };
            let stale_run_id = is_active_bulk_load_status(bulk.status).then_some(bulk.run_id);
            let targets = select_friend_profile_bulk_load_targets(&snapshot.friends_by_id);
            let (total, processed) = friend_profile_bulk_load_initial_progress(
                snapshot.friends_by_id.len(),
                targets.len(),
            );
            let run_id = bulk.run_id.saturating_add(1);
            let now = chrono::Utc::now().to_rfc3339();
            bulk.run_id = run_id;
            bulk.owner = Some(active.clone());
            bulk.total = total;
            bulk.processed = processed;
            bulk.loaded = 0;
            bulk.failed = 0;
            bulk.started_at = now.clone();
            bulk.finished_at = None;
            let spawn_worker = !targets.is_empty();
            bulk.status = if spawn_worker {
                FriendProfileBulkLoadStatus::Running
            } else {
                bulk.finished_at = Some(now);
                FriendProfileBulkLoadStatus::Completed
            };
            (targets, run_id, spawn_worker, stale_run_id)
        };

        if let Some(stale_run_id) = stale_run_id {
            self.friend_profile_bulk_cancel_tx
                .send_replace(stale_run_id);
        }
        let payload = self.emit_friend_profile_bulk_load_status();
        if spawn_worker {
            let runtime = Arc::clone(self);
            self.deps.tasks.spawn(async move {
                runtime
                    .run_friend_profile_bulk_load(run_id, active, targets)
                    .await;
            });
        }
        Ok(payload)
    }

    pub fn cancel_friend_profile_bulk_load(&self) -> Result<FriendProfileLoadStatusPayload> {
        let cancelled_run_id = {
            let mut bulk = self.friend_profile_bulk_load.lock().map_err(|error| {
                Error::Custom(format!("friend profile bulk load lock: {error}"))
            })?;
            if bulk.status == FriendProfileBulkLoadStatus::Running {
                bulk.status = FriendProfileBulkLoadStatus::Cancelling;
                Some(bulk.run_id)
            } else {
                None
            }
        };
        if let Some(run_id) = cancelled_run_id {
            self.friend_profile_bulk_cancel_tx.send_replace(run_id);
        }
        Ok(self.emit_friend_profile_bulk_load_status())
    }

    pub(super) fn cancel_friend_profile_bulk_load_for_session(
        &self,
        active: &ActiveRealtimeContext,
    ) {
        let cancelled_run_id = {
            let Ok(mut bulk) = self.friend_profile_bulk_load.lock() else {
                return;
            };
            if !is_active_bulk_load_status(bulk.status)
                || !friend_profile_bulk_load_owner_matches(bulk.owner.as_ref(), active)
            {
                return;
            }
            bulk.status = FriendProfileBulkLoadStatus::Cancelled;
            bulk.finished_at = Some(chrono::Utc::now().to_rfc3339());
            bulk.run_id
        };
        self.friend_profile_bulk_cancel_tx
            .send_replace(cancelled_run_id);
        self.emit_friend_profile_bulk_load_status();
    }

    pub fn friend_profile_bulk_load_status(&self) -> FriendProfileLoadStatusPayload {
        self.friend_profile_bulk_load
            .lock()
            .map(|bulk| bulk.payload())
            .unwrap_or_default()
    }

    fn emit_friend_profile_bulk_load_status(&self) -> FriendProfileLoadStatusPayload {
        let payload = {
            let Ok(bulk) = self.friend_profile_bulk_load.lock() else {
                return FriendProfileBulkLoadState::default().payload();
            };
            bulk.payload()
        };
        self.deps
            .event_bus
            .emit_friend_profile_load_status(payload.clone());
        payload
    }

    fn friend_profile_bulk_load_is_current(
        &self,
        run_id: u64,
        active: &ActiveRealtimeContext,
    ) -> bool {
        if *self.friend_profile_bulk_cancel_tx.borrow() == run_id {
            return false;
        }
        let bulk_current = self
            .friend_profile_bulk_load
            .lock()
            .map(|bulk| {
                bulk.run_id == run_id
                    && bulk.status == FriendProfileBulkLoadStatus::Running
                    && friend_profile_bulk_load_owner_matches(bulk.owner.as_ref(), active)
            })
            .unwrap_or(false);
        if !bulk_current {
            return false;
        }
        self.state
            .lock()
            .map(|state| {
                self.is_message_current_locked(
                    &state,
                    active.generation,
                    active.session_generation,
                    &active.session,
                )
            })
            .unwrap_or(false)
    }

    async fn load_friend_profile_bulk_item(
        self: &Arc<Self>,
        run_id: u64,
        active: &ActiveRealtimeContext,
        user_id: &str,
        cancel_rx: &mut tokio::sync::watch::Receiver<u64>,
    ) -> Option<(bool, bool)> {
        let mut attempt = 0u32;
        loop {
            if !self.friend_profile_bulk_load_is_current(run_id, active) {
                return None;
            }
            let response = tokio::select! {
                biased;
                _ = wait_for_friend_profile_bulk_load_cancel(run_id, cancel_rx) => return None,
                response = self.get_user_via_cache(
                    active.session.endpoint.clone(),
                    user_id.to_string(),
                    false,
                    false,
                    Some(true),
                ) => response,
            };
            match response {
                Ok(response) if (200..300).contains(&response.status) => {
                    return Some((true, false));
                }
                Ok(response)
                    if response.status == 429 && attempt < FRIEND_PROFILE_BULK_LOAD_MAX_RETRIES =>
                {
                    let delay_ms = friend_profile_bulk_load_backoff_delay_ms(attempt);
                    attempt += 1;
                    tokio::select! {
                        biased;
                        _ = wait_for_friend_profile_bulk_load_cancel(run_id, cancel_rx) => return None,
                        _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
                    }
                    if !self.friend_profile_bulk_load_is_current(run_id, active) {
                        return None;
                    }
                }
                _ => return Some((false, true)),
            }
        }
    }

    fn friend_profile_bulk_load_record_progress(
        &self,
        run_id: u64,
        active: &ActiveRealtimeContext,
        loaded: bool,
        failed: bool,
    ) -> bool {
        if !self.friend_profile_bulk_load_is_current(run_id, active) {
            return false;
        }
        {
            let Ok(mut bulk) = self.friend_profile_bulk_load.lock() else {
                return false;
            };
            if bulk.run_id != run_id || bulk.status != FriendProfileBulkLoadStatus::Running {
                return false;
            }
            bulk.processed = bulk.processed.saturating_add(1);
            if loaded {
                bulk.loaded = bulk.loaded.saturating_add(1);
            }
            if failed {
                bulk.failed = bulk.failed.saturating_add(1);
            }
        }
        self.emit_friend_profile_bulk_load_status();
        true
    }

    async fn run_friend_profile_bulk_load(
        self: Arc<Self>,
        run_id: u64,
        active: ActiveRealtimeContext,
        targets: Vec<String>,
    ) {
        let mut cancel_rx = self.friend_profile_bulk_cancel_tx.subscribe();
        for (index, user_id) in targets.iter().enumerate() {
            if !self.friend_profile_bulk_load_is_current(run_id, &active) {
                break;
            }
            if index > 0 {
                tokio::select! {
                    biased;
                    _ = wait_for_friend_profile_bulk_load_cancel(run_id, &mut cancel_rx) => break,
                    _ = tokio::time::sleep(Duration::from_millis(
                        FRIEND_PROFILE_BULK_LOAD_REQUEST_INTERVAL_MS,
                    )) => {}
                }
                if !self.friend_profile_bulk_load_is_current(run_id, &active) {
                    break;
                }
            }
            let Some((loaded, failed)) = self
                .load_friend_profile_bulk_item(run_id, &active, user_id, &mut cancel_rx)
                .await
            else {
                break;
            };
            if !self.friend_profile_bulk_load_record_progress(run_id, &active, loaded, failed) {
                break;
            }
        }

        self.finish_friend_profile_bulk_load(run_id, &active);
    }

    fn finish_friend_profile_bulk_load(&self, run_id: u64, active: &ActiveRealtimeContext) {
        let session_current = self
            .state
            .lock()
            .map(|state| {
                self.is_message_current_locked(
                    &state,
                    active.generation,
                    active.session_generation,
                    &active.session,
                )
            })
            .unwrap_or(false);
        {
            let Ok(mut bulk) = self.friend_profile_bulk_load.lock() else {
                return;
            };
            if bulk.run_id != run_id {
                return;
            }
            bulk.status =
                if !session_current || bulk.status == FriendProfileBulkLoadStatus::Cancelling {
                    FriendProfileBulkLoadStatus::Cancelled
                } else {
                    FriendProfileBulkLoadStatus::Completed
                };
            bulk.finished_at = Some(chrono::Utc::now().to_rfc3339());
        }
        self.emit_friend_profile_bulk_load_status();
    }
}

#[cfg(test)]
impl RealtimeHostRuntime {
    pub(super) fn test_force_friend_profile_bulk_load_running(&self, run_id: u64, total: u32) {
        let owner = self.state.lock().unwrap().connection.active_context.clone();
        let mut bulk = self.friend_profile_bulk_load.lock().unwrap();
        bulk.run_id = run_id;
        bulk.status = FriendProfileBulkLoadStatus::Running;
        bulk.owner = owner;
        bulk.total = total;
        bulk.started_at = chrono::Utc::now().to_rfc3339();
    }

    pub(super) fn test_friend_profile_bulk_load_is_current(
        &self,
        run_id: u64,
        active: &ActiveRealtimeContext,
    ) -> bool {
        self.friend_profile_bulk_load_is_current(run_id, active)
    }

    pub(super) fn test_friend_profile_bulk_load_record_progress(
        &self,
        run_id: u64,
        loaded: bool,
        failed: bool,
    ) -> bool {
        let Some(active) = self.state.lock().unwrap().connection.active_context.clone() else {
            return false;
        };
        self.friend_profile_bulk_load_record_progress(run_id, &active, loaded, failed)
    }
}

async fn wait_for_friend_profile_bulk_load_cancel(
    run_id: u64,
    cancel_rx: &mut tokio::sync::watch::Receiver<u64>,
) {
    loop {
        if *cancel_rx.borrow_and_update() == run_id {
            return;
        }
        if cancel_rx.changed().await.is_err() {
            return;
        }
    }
}
