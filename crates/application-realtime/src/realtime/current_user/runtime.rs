use std::sync::Mutex;

use serde_json::{json, Map, Value};
use vrcx_0_core::json::text_of;
use vrcx_0_core::realtime::RealtimeWsMessagePayload;

use crate::realtime::event_kind::RealtimeWsEventKind;
use crate::realtime::{
    PendingOfflineTimerAction, RealtimeCurrentUserAuthority, RealtimeCurrentUserOutput,
    RealtimeCurrentUserProjection,
};

use super::avatar::apply_avatar_wear_transition;
use super::game_log::close_remote_game_log_interval;
use super::patch::{
    apply_current_user_patch, apply_user_location, apply_user_update,
    merge_preserved_remote_presence,
};
use super::state::{
    CurrentUserPatchOptions, RealtimeCurrentUserState, RealtimeCurrentUserStateSnapshot,
    CURRENT_USER_REFRESH_LOCAL_AUTHORITY_FIELDS,
};
use super::utils::{has_remote_current_user_presence, map_from_json, normalize_id, EventTime};

#[derive(Debug, Default)]
pub struct RealtimeCurrentUserRuntime {
    state: Mutex<RealtimeCurrentUserState>,
}

impl RealtimeCurrentUserRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_snapshot(
        &self,
        current_user_id: String,
        generation: u64,
        snapshot: serde_json::Value,
    ) {
        let mut state = self.lock_state();
        let current_user_id = normalize_id(&current_user_id);
        let preserves_remote_interval = state.current_user_id == current_user_id;
        state.current_user_id = current_user_id;
        state.generation = generation;
        let mut snapshot =
            RealtimeCurrentUserStateSnapshot::from_value(snapshot, &state.current_user_id);
        if preserves_remote_interval
            && state.remote_game_log_interval.is_some()
            && !has_remote_current_user_presence(&snapshot)
        {
            snapshot = merge_preserved_remote_presence(snapshot, &state.remote_snapshot);
        }
        state.sequence = state.sequence.saturating_add(1);
        state.snapshot = snapshot.clone();
        state.remote_snapshot = snapshot;
        state.pending_offline = None;
        if !preserves_remote_interval {
            state.next_pending_token = 0;
            state.remote_game_log_interval = None;
        }
    }

    pub fn clear(&self) {
        let mut state = self.lock_state();
        state.generation = state.generation.saturating_add(1);
        state.current_user_id.clear();
        state.snapshot = RealtimeCurrentUserStateSnapshot::default();
        state.remote_snapshot = RealtimeCurrentUserStateSnapshot::default();
        state.pending_offline = None;
        state.remote_game_log_interval = None;
    }

    pub fn snapshot_value(&self) -> Option<serde_json::Value> {
        let state = self.lock_state();
        if state.current_user_id.is_empty() {
            return None;
        }
        Some(serde_json::Value::Object(state.snapshot.to_map()))
    }

    #[cfg(test)]
    pub fn apply_ws_message(
        &self,
        generation: u64,
        payload: &RealtimeWsMessagePayload,
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        let event_kind = RealtimeWsEventKind::from_payload(payload)?;
        self.apply_ws_event(generation, &event_kind, payload, authority)
    }

    pub(crate) fn apply_ws_event(
        &self,
        generation: u64,
        event_kind: &RealtimeWsEventKind,
        payload: &RealtimeWsMessagePayload,
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        let content = payload.json.get("content").unwrap_or(&Value::Null);
        let now = EventTime::from_received_at(&payload.received_at);
        let mut state = self.lock_state();
        if state.generation != generation || state.current_user_id.is_empty() {
            return None;
        }

        match event_kind {
            RealtimeWsEventKind::UserUpdate => {
                apply_user_update(&mut state, content, &now, &authority)
            }
            RealtimeWsEventKind::UserLocation => {
                apply_user_location(&mut state, content, &now, &authority)
            }
            _ => None,
        }
    }

    pub fn snapshot_sequence(&self, generation: u64) -> Option<u64> {
        let state = self.lock_state();
        if state.generation != generation || state.current_user_id.is_empty() {
            return None;
        }
        Some(state.sequence)
    }

    pub fn apply_refreshed_snapshot(
        &self,
        generation: u64,
        snapshot: serde_json::Value,
        overlay_patch: serde_json::Value,
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        self.apply_refreshed_snapshot_inner(
            generation,
            None,
            snapshot,
            overlay_patch,
            &[],
            authority,
        )
    }

    pub fn apply_refreshed_snapshot_if_sequence(
        &self,
        generation: u64,
        expected_sequence: u64,
        snapshot: serde_json::Value,
        overlay_patch: serde_json::Value,
        response_authority_fields: &[&str],
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        self.apply_refreshed_snapshot_inner(
            generation,
            Some(expected_sequence),
            snapshot,
            overlay_patch,
            response_authority_fields,
            authority,
        )
    }

    fn apply_refreshed_snapshot_inner(
        &self,
        generation: u64,
        expected_sequence: Option<u64>,
        snapshot: serde_json::Value,
        overlay_patch: serde_json::Value,
        response_authority_fields: &[&str],
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        let mut state = self.lock_state();
        if state.generation != generation || state.current_user_id.is_empty() {
            return None;
        }
        if expected_sequence.is_some_and(|expected_sequence| state.sequence != expected_sequence) {
            return None;
        }
        let event_user_id = snapshot
            .get("id")
            .map(|value| normalize_id(&text_of(Some(value))))
            .unwrap_or_default();
        if event_user_id != state.current_user_id {
            return None;
        }
        let mut patch = snapshot.as_object().cloned().unwrap_or_default();
        remove_current_user_refresh_local_authority_fields(&mut patch, response_authority_fields);
        if let Some(overlay) = overlay_patch.as_object() {
            for (key, value) in overlay {
                patch.insert(key.clone(), value.clone());
            }
        }
        apply_current_user_patch(
            &mut state,
            patch,
            &EventTime::now(),
            &authority,
            CurrentUserPatchOptions {
                applies_local_game_authority: true,
                ..CurrentUserPatchOptions::default()
            },
        )
    }

    pub fn apply_game_running_state(
        &self,
        generation: u64,
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        if !authority.is_available() {
            return None;
        }
        let mut state = self.lock_state();
        if state.generation != generation || state.current_user_id.is_empty() {
            return None;
        }
        if authority.is_game_running() {
            state.pending_offline = None;
        }
        apply_current_user_patch(
            &mut state,
            Map::new(),
            &EventTime::now(),
            &authority,
            CurrentUserPatchOptions {
                applies_local_game_authority: true,
                reconciles_remote_location: !authority.is_game_running(),
                records_current_avatar_history: authority.is_game_running(),
                ..CurrentUserPatchOptions::default()
            },
        )
    }

    pub fn fire_pending_offline(
        &self,
        generation: u64,
        token: u64,
        now: String,
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        let mut state = self.lock_state();
        if state.generation != generation
            || state.current_user_id.is_empty()
            || authority.is_game_running()
            || state.pending_offline.as_ref().map(|pending| pending.token) != Some(token)
        {
            return None;
        }
        let pending = state.pending_offline.take()?;
        apply_current_user_patch(
            &mut state,
            pending.patch,
            &EventTime::from_received_at(&now),
            &authority,
            CurrentUserPatchOptions {
                reconciles_remote_location: true,
                ..CurrentUserPatchOptions::default()
            },
        )
    }

    pub fn interrupt_transport(
        &self,
        generation: u64,
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        self.transport_end_output(generation, authority, false)
    }

    pub fn finalize_transport(
        &self,
        generation: u64,
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        self.transport_end_output(generation, authority, true)
    }

    fn transport_end_output(
        &self,
        generation: u64,
        authority: RealtimeCurrentUserAuthority,
        ends_remote_interval: bool,
    ) -> Option<RealtimeCurrentUserOutput> {
        if !authority.is_available() {
            return None;
        }
        let mut state = self.lock_state();
        if state.generation != generation || state.current_user_id.is_empty() {
            return None;
        }
        let previous = state.snapshot.clone();
        let now = EventTime::now();
        let stopped_authority = authority.with_game_running(false);
        let (snapshot, mut persistence) = apply_avatar_wear_transition(
            previous.clone(),
            &previous,
            &stopped_authority,
            &now,
            false,
        );
        if ends_remote_interval {
            close_remote_game_log_interval(&mut state, &now, &mut persistence);
        }
        let previous_avatar_swap_time = snapshot.previous_avatar_swap_time;
        state.sequence = state.sequence.saturating_add(1);
        state.snapshot = snapshot.clone();
        state.remote_snapshot.set_previous_avatar_swap_time(
            (previous_avatar_swap_time > 0).then_some(previous_avatar_swap_time),
        );
        Some(RealtimeCurrentUserOutput {
            owner_user_id: state.current_user_id.clone(),
            projection: RealtimeCurrentUserProjection {
                generation: state.generation,
                patch: map_from_json(json!({ "id": state.current_user_id.clone() })),
                snapshot: snapshot.to_map(),
                game_state_patch: None,
            },
            persistence,
            timer_action: PendingOfflineTimerAction::None,
        })
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, RealtimeCurrentUserState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}

fn remove_current_user_refresh_local_authority_fields(
    patch: &mut Map<String, Value>,
    response_authority_fields: &[&str],
) {
    for field in CURRENT_USER_REFRESH_LOCAL_AUTHORITY_FIELDS {
        if response_authority_fields.contains(field) {
            continue;
        }
        patch.remove(*field);
    }
}
