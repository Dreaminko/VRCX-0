use std::sync::atomic::Ordering;
use std::sync::Arc;
use vrcx_0_application_core::{Result, RuntimeOperationStatus};

use super::state::FriendOwnerGuard;
use serde_json::Value;
use vrcx_0_application_core::LocalGameContextSnapshot;
use vrcx_0_core::user_facts::UserFactMergeOptions;
use vrcx_0_persistence::config as config_store;
use vrcx_0_persistence::realtime::write_realtime_batch;

use crate::realtime::{
    FriendProjection, PendingOfflineTimerAction, RealtimeCurrentUserOutput, RealtimeFriendOutput,
    RealtimeInstanceClosedOutput, RealtimeNotificationOutput, RealtimeSessionContext,
};

use super::RealtimeHostRuntime;

pub(super) enum FriendOutputApplyOutcome {
    Stale,
    Applied { persistence_succeeded: bool },
}

impl RealtimeHostRuntime {
    pub fn set_feed_persistence_disabled(&self, disabled: bool) -> Result<()> {
        let _owner = self.lock_friend_owner();
        config_store::set_bool(self.deps.db.as_ref(), "feedPersistenceDisabled", disabled)?;
        self.feed_persistence_disabled
            .store(disabled, Ordering::Relaxed);
        self.reset_feed_live_cache();
        Ok(())
    }

    pub(super) fn set_activity_friend_user_ids(&self, user_ids: Vec<String>) {
        if let Some(activity_sink) = &self.deps.activity_sink {
            activity_sink.set_friend_user_ids(user_ids);
        }
    }

    pub(super) fn lock_friend_owner(&self) -> FriendOwnerGuard<'_> {
        FriendOwnerGuard {
            _guard: self
                .friend_owner_lock
                .lock()
                .unwrap_or_else(|error| error.into_inner()),
        }
    }

    #[cfg(test)]
    pub(super) fn apply_friend_output(self: &Arc<Self>, output: RealtimeFriendOutput) {
        let owner = self.lock_friend_owner();
        self.apply_friend_output_owned(&owner, output);
    }

    pub(super) fn apply_reconciled_friend_feed_entries_owned(
        self: &Arc<Self>,
        _owner: &FriendOwnerGuard<'_>,
        owner_user_id: &str,
        generation: u64,
        baseline_revision: u64,
        feed_entries: Vec<Value>,
    ) {
        if feed_entries.is_empty() {
            return;
        }
        let mut projection = FriendProjection::new(generation, baseline_revision);
        projection.feed_entries = feed_entries;
        if !self.is_friend_projection_current(&projection) {
            self.friends
                .clear_baseline_if_revision(projection.generation, projection.baseline_revision);
            return;
        }
        if let Some(activity_sink) = &self.deps.activity_sink {
            activity_sink.ingest_friend_projection(&projection);
        }
        self.emit_feed_entries(
            generation,
            owner_user_id,
            std::mem::take(&mut projection.feed_entries),
        );
    }

    pub(super) fn apply_friend_output_owned(
        self: &Arc<Self>,
        _owner: &FriendOwnerGuard<'_>,
        mut output: RealtimeFriendOutput,
    ) -> FriendOutputApplyOutcome {
        let timer_action = output.timer_action.clone();
        let profile_refetch_user_ids = output.profile_refetch_user_ids.clone();
        let mut projection = output.projection.clone();
        let projection_generation = projection.generation;
        if !self.is_friend_projection_current(&projection) {
            self.friends
                .clear_baseline_if_revision(projection.generation, projection.baseline_revision);
            return FriendOutputApplyOutcome::Stale;
        }
        self.retain_current_instance_joining_entries(&mut projection, &output.owner_user_id);
        let feed_persistence_disabled = self.feed_persistence_disabled.load(Ordering::Relaxed);
        if feed_persistence_disabled {
            output.persistence.feed_entries.clear();
        }
        let friend_note_changed = output.friend_note_changed;
        let mut world_name_fetch_ids =
            self.enrich_projection_world_names(&mut projection.feed_entries);
        world_name_fetch_ids.extend(self.enrich_persistence_world_names(&mut output.persistence));
        let persisted =
            match write_realtime_batch(&self.deps.db, &output.owner_user_id, &output.persistence) {
                Ok(_) => {
                    self.deps.sync.record(
                        "realtimeFriends",
                        RuntimeOperationStatus::Persisted,
                        "Realtime friend projection persisted by Rust.",
                        0,
                    );
                    true
                }
                Err(error) => {
                    tracing::warn!("Realtime friend persistence failed: {error}");
                    self.deps
                        .sync
                        .record_failure("realtimeFriends", error.to_string());
                    if !feed_persistence_disabled {
                        projection.feed_entries.clear();
                    }
                    false
                }
            };
        if let Some(activity_sink) = &self.deps.activity_sink {
            activity_sink.ingest_friend_projection(&projection);
        }
        projection
            .feed_entries
            .retain(|entry| !is_player_joining_entry(entry));
        let feed_entries = std::mem::take(&mut projection.feed_entries);
        if friend_note_changed {
            if let Some(sink) = &self.deps.friend_note_change_sink {
                sink();
            }
        }
        if !projection.patches.is_empty() {
            let changed = self.collect_friend_record_cache_changes(
                projection.patches.iter().map(|patch| &patch.patch),
                &UserFactMergeOptions {
                    endpoint: self.active_endpoint(),
                    source: "realtime".into(),
                    received_at: chrono::Utc::now().to_rfc3339(),
                    is_friend: true,
                    ..Default::default()
                },
            );
            self.emit_user_cache_changes(changed);
        }
        self.deps
            .event_bus
            .emit_realtime_friend_projection(projection);
        self.emit_feed_entries(projection_generation, &output.owner_user_id, feed_entries);

        if let PendingOfflineTimerAction::Schedule {
            user_id,
            token,
            delay_ms,
        } = timer_action
        {
            let runtime = Arc::clone(self);
            self.deps.tasks.spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                let now = chrono::Utc::now().to_rfc3339();
                runtime.fire_pending_offline(&user_id, token, now);
            });
        }
        self.schedule_friend_profile_refetches(projection_generation, profile_refetch_user_ids);
        self.schedule_world_name_warm(world_name_fetch_ids);
        FriendOutputApplyOutcome::Applied {
            persistence_succeeded: persisted,
        }
    }

    fn retain_current_instance_joining_entries(
        &self,
        projection: &mut FriendProjection,
        current_user_id: &str,
    ) {
        if !projection.feed_entries.iter().any(is_player_joining_entry) {
            return;
        }
        let local_game_context = self.deps.local_game_context.snapshot();
        let (is_game_running, current_location, player_user_ids) = match &local_game_context {
            LocalGameContextSnapshot::Unavailable => (false, "", &[][..]),
            LocalGameContextSnapshot::Available {
                is_game_running,
                location,
                player_user_ids,
                ..
            } => (
                *is_game_running,
                location.trim(),
                player_user_ids.as_slice(),
            ),
        };
        let current_user_id = current_user_id.trim();
        projection.feed_entries.retain(|entry| {
            if !is_player_joining_entry(entry) {
                return true;
            }
            let user_id = entry
                .get("userId")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let destination = entry
                .get("travelingToLocation")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            is_game_running
                && !current_location.is_empty()
                && destination == current_location
                && !user_id.is_empty()
                && user_id != current_user_id
                && !player_user_ids
                    .iter()
                    .any(|player_user_id| player_user_id == user_id)
        });
    }

    pub(super) fn apply_notification_output(
        self: &Arc<Self>,
        mut output: RealtimeNotificationOutput,
    ) {
        let mut projection = output.projection;
        let mut world_name_fetch_ids = self.enrich_notification_world_names(&mut projection);
        self.enrich_notification_sender_names(&mut projection);
        self.enrich_notification_images(&mut projection, &output.owner_user_id);
        world_name_fetch_ids.extend(self.enrich_persistence_world_names(&mut output.persistence));
        self.enrich_persistence_sender_names(&mut output.persistence);
        output.projection = projection;
        self.finalize_notification_output_for_delivery(&mut output);
        let projection = self.visible_notification_projection(output.projection.clone());
        match write_realtime_batch(&self.deps.db, &output.owner_user_id, &output.persistence) {
            Ok(_) => {
                self.deps.sync.record(
                    "realtimeNotifications",
                    RuntimeOperationStatus::Persisted,
                    "Realtime notification projection persisted by Rust.",
                    0,
                );
            }
            Err(error) => {
                tracing::warn!("Realtime notification persistence failed: {error}");
                self.deps
                    .sync
                    .record_failure("realtimeNotifications", error.to_string());
            }
        }
        if self.projection_has_visible_notification_work(&projection) {
            if let Some(activity_sink) = &self.deps.activity_sink {
                activity_sink.ingest_notification_projection(&projection);
            }
            self.deps
                .event_bus
                .emit_realtime_notification_projection(projection.clone());
            self.schedule_invite_automation(&projection);
        }
        self.schedule_world_name_warm(world_name_fetch_ids);
    }

    pub(super) fn schedule_notification_output(
        self: &Arc<Self>,
        generation: u64,
        session_generation: u64,
        session: RealtimeSessionContext,
        output: RealtimeNotificationOutput,
    ) {
        let runtime = Arc::clone(self);
        self.deps.tasks.spawn(async move {
            let _guard = runtime.notification_apply_lock.lock().await;
            if !runtime.is_notification_context_current(generation, session_generation, &session) {
                return;
            }
            let mut output = output;
            if runtime.notification_output_needs_remote_resolution(&output) {
                runtime.resolve_notification_output_names(&mut output).await;
                if !runtime.is_notification_context_current(
                    generation,
                    session_generation,
                    &session,
                ) {
                    return;
                }
            }
            runtime.apply_notification_output(output);
        });
    }

    pub(super) fn apply_current_user_output(&self, mut output: RealtimeCurrentUserOutput) {
        self.enrich_current_user_location_output(&mut output);
        let projection = output.projection;
        match write_realtime_batch(&self.deps.db, &output.owner_user_id, &output.persistence) {
            Ok(_) => {
                self.deps.sync.record(
                    "realtimeCurrentUser",
                    RuntimeOperationStatus::Persisted,
                    "Realtime current-user projection persisted by Rust.",
                    0,
                );
            }
            Err(error) => {
                tracing::warn!("Realtime current user persistence failed: {error}");
                self.deps
                    .sync
                    .record_failure("realtimeCurrentUser", error.to_string());
            }
        }
        self.deps
            .event_bus
            .emit_realtime_current_user_projection(projection);
    }

    pub(super) fn apply_instance_closed_output(
        &self,
        owner_user_id: &str,
        output: RealtimeInstanceClosedOutput,
    ) {
        let mut projection = output.projection;
        let mut feed_entry = output.feed_entry;
        let generation = projection.generation;
        self.enrich_world_name(&mut projection.notification);
        self.enrich_world_name(&mut feed_entry);
        if let Some(location) = projection
            .notification
            .get("location")
            .and_then(Value::as_str)
        {
            if let Ok(mut state) = self.state.lock() {
                state.automation.invite.record_closed_location(location);
            }
        }
        match write_realtime_batch(&self.deps.db, owner_user_id, &output.persistence) {
            Ok(_) => {
                self.deps.sync.record(
                    "realtimeInstanceClosed",
                    RuntimeOperationStatus::Persisted,
                    "Realtime instance-closed projection persisted by Rust.",
                    0,
                );
            }
            Err(error) => {
                tracing::warn!("Realtime instance-closed persistence failed: {error}");
                self.deps
                    .sync
                    .record_failure("realtimeInstanceClosed", error.to_string());
            }
        }
        if let Some(activity_sink) = &self.deps.activity_sink {
            activity_sink.ingest_instance_closed_projection(&projection);
        }
        self.deps
            .event_bus
            .emit_realtime_instance_closed_projection(projection);
        self.emit_feed_entries(generation, owner_user_id, vec![feed_entry]);
    }
}

fn is_player_joining_entry(entry: &Value) -> bool {
    entry.get("type").and_then(Value::as_str) == Some("OnPlayerJoining")
}
