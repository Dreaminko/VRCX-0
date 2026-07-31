use serde_json::{Map, Value};
use vrcx_0_core::json::JsonExt;
use vrcx_0_core::text::first_owned;

use crate::realtime::runtime_types::PENDING_OFFLINE_DELAY_MS;
use crate::realtime::{
    PendingOfflineTimerAction, RealtimeCurrentUserAuthority, RealtimeCurrentUserOutput,
    RealtimeCurrentUserProjection,
};

use super::avatar::apply_avatar_wear_transition;
use super::game_log::{
    close_remote_game_log_interval, game_log_authority_patch, reconcile_remote_game_log_interval,
};
use super::location::{build_location_patch, location_game_state_patch};
use super::state::{
    CurrentUserPatchOptions, PendingCurrentUserOffline, RealtimeCurrentUserState,
    RealtimeCurrentUserStateSnapshot, CURRENT_USER_REMOTE_PRESENCE_FIELDS,
};
use super::utils::{
    has_remote_current_user_presence, is_offline_location, normalize_id, resolve_state_bucket,
    EventTime,
};

pub(super) fn apply_user_update(
    state: &mut RealtimeCurrentUserState,
    content: &Value,
    now: &EventTime,
    authority: &RealtimeCurrentUserAuthority,
) -> Option<RealtimeCurrentUserOutput> {
    let mut patch = content
        .get("user")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    patch.remove("state");
    let event_user_id = first_owned([patch.text_field("id"), content.text_field("userId")]);
    if event_user_id != state.current_user_id {
        return None;
    }
    let previous_snapshot = state.snapshot.to_map();
    if let Some(state_bucket) = resolve_state_bucket(content, &patch, Some(&previous_snapshot)) {
        patch.insert("stateBucket".into(), Value::String(state_bucket));
    }
    if patch.is_empty() {
        return None;
    }
    apply_current_user_patch(
        state,
        patch,
        now,
        authority,
        CurrentUserPatchOptions {
            applies_local_game_authority: true,
            ..CurrentUserPatchOptions::default()
        },
    )
}

pub(super) fn apply_user_location(
    state: &mut RealtimeCurrentUserState,
    content: &Value,
    now: &EventTime,
    authority: &RealtimeCurrentUserAuthority,
) -> Option<RealtimeCurrentUserOutput> {
    let event_user_id = normalize_id(&content.text_field("userId"));
    if event_user_id != state.current_user_id {
        return None;
    }
    let patch = build_location_patch(
        content.get("location"),
        content.get("travelingToLocation"),
        content.get("worldId"),
    );
    if authority.is_game_running {
        state.pending_offline = None;
        return apply_current_user_patch(
            state,
            patch,
            now,
            authority,
            CurrentUserPatchOptions {
                applies_local_game_authority: true,
                ..CurrentUserPatchOptions::default()
            },
        );
    }
    if is_offline_location(&patch.text_field("location"))
        && has_remote_current_user_presence(&state.remote_snapshot)
    {
        if state.pending_offline.is_some() {
            return None;
        }
        state.next_pending_token = state.next_pending_token.saturating_add(1);
        let token = state.next_pending_token;
        state.pending_offline = Some(PendingCurrentUserOffline { token, patch });
        return apply_current_user_patch(
            state,
            Map::new(),
            now,
            authority,
            CurrentUserPatchOptions {
                timer_action: PendingOfflineTimerAction::Schedule {
                    user_id: state.current_user_id.clone(),
                    token,
                    delay_ms: PENDING_OFFLINE_DELAY_MS,
                },
                ..CurrentUserPatchOptions::default()
            },
        );
    }
    state.pending_offline = None;
    apply_current_user_patch(
        state,
        patch,
        now,
        authority,
        CurrentUserPatchOptions {
            applies_local_game_authority: true,
            reconciles_remote_location: true,
            ..CurrentUserPatchOptions::default()
        },
    )
}

pub(super) fn apply_current_user_patch(
    state: &mut RealtimeCurrentUserState,
    patch: Map<String, Value>,
    now: &EventTime,
    authority: &RealtimeCurrentUserAuthority,
    options: CurrentUserPatchOptions,
) -> Option<RealtimeCurrentUserOutput> {
    let previous = state.snapshot.clone();
    let mut projection_patch = patch.clone();
    let mut remote_merged = state.remote_snapshot.to_map();
    for (key, value) in &patch {
        remote_merged.insert(key.clone(), value.clone());
    }
    remote_merged.insert("id".into(), Value::String(state.current_user_id.clone()));
    state.remote_snapshot =
        RealtimeCurrentUserStateSnapshot::from_map(remote_merged, &state.current_user_id);

    let mut merged = if authority.is_game_running {
        let mut local_merged = previous.to_map();
        for (key, value) in &patch {
            local_merged.insert(key.clone(), value.clone());
        }
        local_merged
    } else {
        state.remote_snapshot.to_map()
    };
    if options.applies_local_game_authority && authority.is_game_running {
        if let Some(authority_patch) = game_log_authority_patch(authority) {
            for (key, value) in &authority_patch {
                merged.insert(key.clone(), value.clone());
                projection_patch.insert(key.clone(), value.clone());
            }
        }
    }
    merged.insert("id".into(), Value::String(state.current_user_id.clone()));
    normalize_current_user_presence(
        &mut merged,
        authority.is_game_running
            || state.pending_offline.is_some()
            || has_remote_current_user_presence(&state.remote_snapshot),
    );
    projection_patch.insert("id".into(), Value::String(state.current_user_id.clone()));
    let (snapshot, mut persistence) = apply_avatar_wear_transition(
        RealtimeCurrentUserStateSnapshot::from_map(merged, &state.current_user_id),
        &previous,
        authority.local_game_context_available,
        authority.is_game_running,
        now,
        options.records_current_avatar_history,
    );
    projection_patch.insert("state".into(), Value::String(snapshot.state_bucket.clone()));
    projection_patch.insert(
        "stateBucket".into(),
        Value::String(snapshot.state_bucket.clone()),
    );
    if !authority.is_game_running && options.reconciles_remote_location {
        copy_current_user_presence_patch(&snapshot, &mut projection_patch);
    }

    if authority.is_game_running {
        close_remote_game_log_interval(state, now, &mut persistence);
    } else if options.reconciles_remote_location {
        reconcile_remote_game_log_interval(
            state,
            &snapshot,
            now,
            authority.game_log_enabled,
            &mut persistence,
        );
    }

    let writes_location_game_state = authority.local_game_context_available
        && options.reconciles_remote_location
        && !authority.is_game_running;
    let game_state_patch = if writes_location_game_state {
        Some(location_game_state_patch(&snapshot, now))
    } else {
        None
    };

    let snapshot_map = snapshot.to_map();
    state.sequence = state.sequence.saturating_add(1);
    state.snapshot = snapshot;
    Some(RealtimeCurrentUserOutput {
        owner_user_id: state.current_user_id.clone(),
        projection: RealtimeCurrentUserProjection {
            generation: state.generation,
            patch: projection_patch,
            snapshot: snapshot_map,
            game_state_patch,
        },
        persistence,
        timer_action: options.timer_action,
    })
}

fn normalize_current_user_presence(merged: &mut Map<String, Value>, is_online: bool) {
    let state_bucket = if is_online { "online" } else { "active" };
    merged.insert("state".into(), Value::String(state_bucket.into()));
    merged.insert("stateBucket".into(), Value::String(state_bucket.into()));
    merged.remove("pendingOffline");
}

fn copy_current_user_presence_patch(
    snapshot: &RealtimeCurrentUserStateSnapshot,
    projection_patch: &mut Map<String, Value>,
) {
    let snapshot = snapshot.to_map();
    for field in CURRENT_USER_REMOTE_PRESENCE_FIELDS {
        if let Some(value) = snapshot.get(*field) {
            projection_patch.insert((*field).into(), value.clone());
        } else {
            projection_patch.remove(*field);
        }
    }
    projection_patch.remove("pendingOffline");
}

pub(super) fn merge_preserved_remote_presence(
    snapshot: RealtimeCurrentUserStateSnapshot,
    previous: &RealtimeCurrentUserStateSnapshot,
) -> RealtimeCurrentUserStateSnapshot {
    let current_user_id = snapshot.user_id.clone();
    let mut merged = snapshot.to_map();
    let previous = previous.to_map();
    for field in CURRENT_USER_REMOTE_PRESENCE_FIELDS {
        if let Some(value) = previous.get(*field) {
            merged.insert((*field).into(), value.clone());
        }
    }
    RealtimeCurrentUserStateSnapshot::from_map(merged, &current_user_id)
}
