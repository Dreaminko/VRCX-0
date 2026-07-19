use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration as StdDuration;

use serde_json::json;
use vrcx_0_vrchat_client::auth::current_user_get_input;
use vrcx_0_vrchat_client::http_api::ApiScope;
use vrcx_0_vrchat_client::users::user_get_input;

use crate::social_baseline::service::{
    derive_friend_state_buckets, execute_vrchat_json_request, refetch_users_concurrent,
    FriendStateBuckets,
};
use crate::social_baseline::SocialBaselineDeps;

use super::*;

const RECONCILE_STABILITY_WINDOW: StdDuration = StdDuration::from_secs(60);
const RECONCILE_COOLDOWN_SECONDS: i64 = 5 * 60;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct FriendRosterDiff {
    pub(crate) suspicious: Vec<String>,
    pub(crate) added: Vec<String>,
    pub(crate) removed: Vec<String>,
}

impl FriendRosterDiff {
    pub(crate) fn is_empty(&self) -> bool {
        self.suspicious.is_empty() && self.added.is_empty() && self.removed.is_empty()
    }
}

pub(crate) fn diff_friend_roster_against_auth_user(
    buckets: &FriendStateBuckets,
    local_friends_by_id: &HashMap<String, FriendRecord>,
) -> Option<FriendRosterDiff> {
    if !buckets.has_friend_list {
        return None;
    }
    let expected_set: HashSet<&str> = buckets.expected_ids.iter().map(String::as_str).collect();
    let mut suspicious = Vec::new();
    let mut added = Vec::new();
    for user_id in &buckets.expected_ids {
        let api_bucket = buckets
            .state_by_id
            .get(user_id)
            .map(String::as_str)
            .unwrap_or("offline");
        match local_friends_by_id.get(user_id) {
            Some(record) => {
                if local_friend_state_bucket(record) != api_bucket {
                    suspicious.push(user_id.clone());
                }
            }
            None => added.push(user_id.clone()),
        }
    }
    let mut removed: Vec<String> = local_friends_by_id
        .keys()
        .filter(|user_id| !expected_set.contains(user_id.as_str()))
        .cloned()
        .collect();
    removed.sort();
    Some(FriendRosterDiff {
        suspicious,
        added,
        removed,
    })
}

pub(crate) fn local_friend_state_bucket(record: &FriendRecord) -> String {
    vrcx_0_core::friends::normalize_state_bucket(&record.state_bucket)
        .or_else(|| vrcx_0_core::friends::normalize_state_bucket(&record.state))
        .unwrap_or_else(|| "offline".to_string())
}

pub(crate) fn resolve_bucket_message_type(
    local_bucket: &str,
    profile: &Value,
) -> Option<&'static str> {
    let state = profile.get("state").and_then(Value::as_str).unwrap_or("");
    let resolved = vrcx_0_core::friends::normalize_state_bucket(state)?;
    if resolved == local_bucket {
        return None;
    }
    Some(match resolved.as_str() {
        "online" => "friend-online",
        "active" => "friend-active",
        _ => "friend-offline",
    })
}

pub(crate) fn profile_reports_non_friend(profile: &Value) -> bool {
    profile.get("isFriend").and_then(Value::as_bool) == Some(false)
}

pub(crate) fn confirmed_non_friend(status_code: i32, body: Option<&Value>) -> bool {
    if status_code == 404 {
        return true;
    }
    if !(200..300).contains(&status_code) {
        return false;
    }
    matches!(
        body.and_then(|body| body.get("isFriend"))
            .and_then(Value::as_bool),
        Some(false)
    )
}

#[derive(Default, Debug)]
struct ReconcileCounts {
    suspicious: usize,
    added: usize,
    removed: usize,
    corrected: u64,
    skipped_by_sequence: u64,
}

struct ReconcileRunGuard<'a> {
    flag: &'a AtomicBool,
}

impl<'a> ReconcileRunGuard<'a> {
    fn try_acquire(flag: &'a AtomicBool) -> Option<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self { flag })
    }
}

impl Drop for ReconcileRunGuard<'_> {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Release);
    }
}

impl RealtimeHostRuntime {
    pub fn spawn_reconnect_reconcile(self: &Arc<Self>, transport: RealtimeTransportStartResult) {
        let runtime = Arc::clone(self);
        self.deps.tasks.spawn(async move {
            runtime.run_reconnect_reconcile(transport).await;
        });
    }

    async fn run_reconnect_reconcile(self: Arc<Self>, transport: RealtimeTransportStartResult) {
        tokio::time::sleep(RECONCILE_STABILITY_WINDOW).await;
        if !self.transport_is_active(&transport) {
            return;
        }
        if !self.reconcile_cooldown_elapsed(transport.generation) {
            return;
        }
        let Some(_guard) = ReconcileRunGuard::try_acquire(&self.reconcile_running) else {
            return;
        };
        if !self.transport_is_active(&transport) {
            return;
        }
        self.record_reconcile_run(transport.generation);
        self.execute_reconnect_reconcile(transport).await;
    }

    fn reconcile_cooldown_elapsed(&self, generation: u64) -> bool {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        match state.reconcile.last_run {
            Some((last_generation, last_at)) if last_generation == generation => {
                chrono::Utc::now()
                    .signed_duration_since(last_at)
                    .num_seconds()
                    >= RECONCILE_COOLDOWN_SECONDS
            }
            _ => true,
        }
    }

    fn record_reconcile_run(&self, generation: u64) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.reconcile.last_run = Some((generation, chrono::Utc::now()));
    }

    fn active_generation(&self) -> Option<u64> {
        self.state.lock().ok().and_then(|state| {
            state
                .connection
                .active_context
                .as_ref()
                .map(|active| active.generation)
        })
    }

    async fn execute_reconnect_reconcile(
        self: &Arc<Self>,
        transport: RealtimeTransportStartResult,
    ) {
        let Some(snapshot) = self
            .friend_snapshot()
            .filter(|snapshot| snapshot.generation == transport.generation)
        else {
            return;
        };
        let owner_user_id = snapshot.current_user_id.clone();
        let endpoint = snapshot.endpoint.clone();
        let deps = SocialBaselineDeps {
            db: Arc::clone(&self.deps.db),
            web: Arc::clone(&self.deps.web),
            auth_scope: self.deps.auth_scope.clone(),
            session: self.deps.session.clone(),
        };

        let current_user_snapshot = match execute_vrchat_json_request(
            &deps,
            current_user_get_input(endpoint.clone()),
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                tracing::info!(
                    error = %error,
                    "[Realtime] reconnect reconcile /auth/user fetch failed; abandoning"
                );
                return;
            }
        };
        if !self.transport_is_active(&transport) {
            return;
        }

        let buckets = derive_friend_state_buckets(&current_user_snapshot);
        let Some(snapshot) = self
            .friend_snapshot()
            .filter(|snapshot| snapshot.generation == transport.generation)
        else {
            return;
        };
        let Some(diff) = diff_friend_roster_against_auth_user(&buckets, &snapshot.friends_by_id)
        else {
            tracing::info!(
                generation = transport.generation,
                "[Realtime] reconnect reconcile abandoned: /auth/user friend list incomplete"
            );
            return;
        };
        if diff.is_empty() {
            tracing::info!(
                generation = transport.generation,
                "[Realtime] reconnect reconcile found no drift"
            );
            return;
        }
        let mut counts = ReconcileCounts {
            suspicious: diff.suspicious.len(),
            added: diff.added.len(),
            removed: diff.removed.len(),
            ..Default::default()
        };

        let mut refetch_ids = diff.suspicious.clone();
        refetch_ids.extend(diff.added.iter().cloned());
        let refetched = if refetch_ids.is_empty() {
            HashMap::new()
        } else {
            refetch_users_concurrent(&deps, &endpoint, refetch_ids).await
        };
        if !self.transport_is_active(&transport) {
            return;
        }

        for user_id in &diff.suspicious {
            if !self.transport_is_active(&transport) {
                break;
            }
            let Some(profile) = refetched.get(user_id) else {
                continue;
            };
            if profile_reports_non_friend(profile) {
                continue;
            }
            let Some(local_generation) = self.active_generation() else {
                break;
            };
            if local_generation != transport.generation {
                break;
            }
            let Some(local_record) = self
                .friend_snapshot()
                .and_then(|snapshot| snapshot.friends_by_id.get(user_id).cloned())
            else {
                continue;
            };
            let local_bucket = local_friend_state_bucket(&local_record);
            let Some(message_type) = resolve_bucket_message_type(&local_bucket, profile) else {
                continue;
            };
            let expected_sequence = self
                .friends
                .friend_state_sequence_for_user(local_generation, user_id);
            let content = json!({ "userId": user_id, "user": profile });
            self.land_reconcile_correction(
                &owner_user_id,
                &endpoint,
                local_generation,
                user_id,
                expected_sequence,
                message_type,
                content,
                false,
                &mut counts,
            );
        }

        for user_id in &diff.added {
            if !self.transport_is_active(&transport) {
                break;
            }
            let Some(profile) = refetched.get(user_id) else {
                continue;
            };
            if profile_reports_non_friend(profile) {
                continue;
            }
            let Some(local_generation) = self.active_generation() else {
                break;
            };
            if local_generation != transport.generation {
                break;
            }
            let content = json!({ "userId": user_id, "user": profile });
            self.land_reconcile_correction(
                &owner_user_id,
                &endpoint,
                local_generation,
                user_id,
                None,
                "friend-add",
                content,
                true,
                &mut counts,
            );
        }

        for user_id in &diff.removed {
            if !self.transport_is_active(&transport) {
                break;
            }
            let Some((status_code, body)) =
                fetch_user_for_removal_confirmation(&deps, &endpoint, user_id).await
            else {
                continue;
            };
            if !confirmed_non_friend(status_code, body.as_ref()) {
                continue;
            }
            let Some(local_generation) = self.active_generation() else {
                break;
            };
            if local_generation != transport.generation {
                break;
            }
            let Some(expected_sequence) = self
                .friends
                .friend_state_sequence_for_user(local_generation, user_id)
            else {
                continue;
            };
            let content = json!({ "userId": user_id });
            self.land_reconcile_correction(
                &owner_user_id,
                &endpoint,
                local_generation,
                user_id,
                Some(expected_sequence),
                "friend-delete",
                content,
                false,
                &mut counts,
            );
        }

        self.log_reconcile_summary(transport.generation, &counts);
    }

    #[allow(clippy::too_many_arguments)]
    fn land_reconcile_correction(
        self: &Arc<Self>,
        owner_user_id: &str,
        endpoint: &str,
        generation: u64,
        user_id: &str,
        expected_sequence: Option<u64>,
        message_type: &str,
        content: Value,
        trust_friend_add_profile_state: bool,
        counts: &mut ReconcileCounts,
    ) {
        match self.apply_synthetic_friend_event_if_sequence(
            owner_user_id,
            endpoint,
            generation,
            user_id,
            expected_sequence,
            message_type,
            content,
            chrono::Utc::now().to_rfc3339(),
            trust_friend_add_profile_state,
        ) {
            SyntheticFriendEventOutcome::Applied => counts.corrected += 1,
            SyntheticFriendEventOutcome::Ignored | SyntheticFriendEventOutcome::MissingBaseline => {
                counts.skipped_by_sequence += 1;
            }
            SyntheticFriendEventOutcome::PersistFailed => {}
        }
    }

    fn log_reconcile_summary(&self, generation: u64, counts: &ReconcileCounts) {
        let detail = format!(
            "suspicious={} added={} removed={} corrected={} skipped_by_sequence={}",
            counts.suspicious,
            counts.added,
            counts.removed,
            counts.corrected,
            counts.skipped_by_sequence
        );
        if counts.corrected > 0 {
            tracing::warn!(
                generation,
                suspicious = counts.suspicious,
                added = counts.added,
                removed = counts.removed,
                corrected = counts.corrected,
                skipped_by_sequence = counts.skipped_by_sequence,
                "[Realtime] reconnect reconcile corrected friend drift"
            );
        } else {
            tracing::info!(
                generation,
                suspicious = counts.suspicious,
                added = counts.added,
                removed = counts.removed,
                corrected = counts.corrected,
                skipped_by_sequence = counts.skipped_by_sequence,
                "[Realtime] reconnect reconcile completed"
            );
        }
        self.deps
            .sync
            .record("realtimeReconcile", "completed", detail, counts.corrected);
    }
}

async fn fetch_user_for_removal_confirmation(
    deps: &SocialBaselineDeps,
    endpoint: &str,
    user_id: &str,
) -> Option<(i32, Option<Value>)> {
    let (_, request) = user_get_input(endpoint.to_string(), user_id.to_string()).ok()?;
    let response = deps
        .web
        .execute_api(request, ApiScope::Vrchat, deps.db.as_ref())
        .await
        .ok()?;
    let body = serde_json::from_str::<Value>(&response.data).ok();
    Some((response.status, body))
}
