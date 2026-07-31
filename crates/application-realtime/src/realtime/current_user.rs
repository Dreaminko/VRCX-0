use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use vrcx_0_core::json::{text_of, JsonExt};
use vrcx_0_core::location::parse_location;
use vrcx_0_core::realtime::RealtimeWsMessagePayload;
use vrcx_0_core::text::first_owned;
use vrcx_0_persistence::game_log::{GameLogLocationEntry, GameLogLocationTimeUpdate};
use vrcx_0_persistence::realtime::{
    AvatarHistoryUpsert, AvatarTimeSpentUpsert, RealtimePersistenceBatch,
};

use super::runtime_types::PENDING_OFFLINE_DELAY_MS;
use super::{
    PendingOfflineTimerAction, RealtimeCurrentUserAuthority, RealtimeCurrentUserOutput,
    RealtimeCurrentUserProjection,
};

#[derive(Clone, Debug, Default)]
struct RealtimeCurrentUserState {
    generation: u64,
    sequence: u64,
    current_user_id: String,
    snapshot: RealtimeCurrentUserStateSnapshot,
    remote_snapshot: RealtimeCurrentUserStateSnapshot,
    pending_offline: Option<PendingCurrentUserOffline>,
    next_pending_token: u64,
    remote_game_log_interval: Option<RemoteGameLogInterval>,
}

#[derive(Clone, Debug)]
struct PendingCurrentUserOffline {
    token: u64,
    patch: Map<String, Value>,
}

#[derive(Clone, Debug)]
struct RemoteGameLogInterval {
    created_at: String,
    started_at_ms: i64,
    location: String,
}

#[derive(Default)]
struct CurrentUserPatchOptions {
    applies_local_game_authority: bool,
    reconciles_remote_location: bool,
    records_current_avatar_history: bool,
    timer_action: PendingOfflineTimerAction,
}

#[derive(Clone, Debug, Default)]
struct RealtimeCurrentUserStateSnapshot {
    raw: Map<String, Value>,
    user_id: String,
    display_name: String,
    location: String,
    traveling_to_location: String,
    world_id: String,
    instance_id: String,
    status: String,
    status_description: String,
    bio: String,
    current_avatar: String,
    current_avatar_image_url: String,
    state_bucket: String,
    world_name: String,
    previous_avatar_swap_time: i64,
}

impl RealtimeCurrentUserStateSnapshot {
    fn from_value(snapshot: serde_json::Value, current_user_id: &str) -> Self {
        Self::from_map(
            snapshot.as_object().cloned().unwrap_or_default(),
            current_user_id,
        )
    }

    fn from_map(mut raw: Map<String, Value>, current_user_id: &str) -> Self {
        if !current_user_id.is_empty() {
            raw.insert("id".into(), Value::String(current_user_id.to_string()));
        }
        let mut snapshot = Self {
            raw,
            ..Self::default()
        };
        snapshot.refresh_typed_fields();
        snapshot
    }

    fn to_map(&self) -> Map<String, Value> {
        let mut raw = self.raw.clone();
        if !self.user_id.is_empty() {
            raw.insert("id".into(), Value::String(self.user_id.clone()));
        }
        raw
    }

    fn set_previous_avatar_swap_time(&mut self, value: Option<i64>) {
        self.previous_avatar_swap_time = value.unwrap_or_default();
        self.raw.insert(
            "$previousAvatarSwapTime".into(),
            value.map(Value::from).unwrap_or(Value::Null),
        );
    }

    fn refresh_typed_fields(&mut self) {
        self.user_id = normalize_id(&self.raw.text_field("id"));
        self.display_name = self.raw.text_field("displayName");
        self.location = self.raw.text_field("location");
        self.traveling_to_location = self.raw.text_field("travelingToLocation");
        self.world_id = self.raw.text_field("worldId");
        self.instance_id = self.raw.text_field("instanceId");
        self.status = self.raw.text_field("status");
        self.status_description = self.raw.text_field("statusDescription");
        self.bio = self.raw.text_field("bio");
        self.current_avatar = normalize_id(&self.raw.text_field("currentAvatar"));
        self.current_avatar_image_url = self.raw.text_field("currentAvatarImageUrl");
        self.state_bucket = self.raw.text_field("stateBucket");
        self.world_name = self.raw.text_field("worldName");
        self.previous_avatar_swap_time = self
            .raw
            .i64_field("$previousAvatarSwapTime")
            .unwrap_or_default();
    }
}

const CURRENT_USER_REFRESH_LOCAL_AUTHORITY_FIELDS: &[&str] = &[
    "friends",
    "onlineFriends",
    "activeFriends",
    "offlineFriends",
    "status",
    "statusDescription",
    "state",
    "stateBucket",
    "pendingOffline",
    "location",
    "$location",
    "$location_at",
    "locationUpdatedAt",
    "worldId",
    "instanceId",
    "travelingToLocation",
    "travelingToWorld",
    "travelingToInstance",
    "$travelingToLocation",
    "$travelingToTime",
    "travelingToTime",
    "$previousLocation",
    "$previousLocation_at",
];

pub const CURRENT_USER_AVATAR_RESPONSE_AUTHORITY_FIELDS: &[&str] = &[
    "currentAvatar",
    "currentAvatarImageUrl",
    "currentAvatarName",
    "currentAvatarTags",
    "currentAvatarThumbnailImageUrl",
];

pub const CURRENT_USER_FALLBACK_AVATAR_RESPONSE_AUTHORITY_FIELDS: &[&str] = &["fallbackAvatar"];

const CURRENT_USER_REMOTE_PRESENCE_FIELDS: &[&str] = &[
    "location",
    "$location",
    "$location_at",
    "locationUpdatedAt",
    "worldId",
    "instanceId",
    "travelingToLocation",
    "travelingToWorld",
    "travelingToInstance",
    "$travelingToLocation",
    "$travelingToTime",
    "worldName",
    "state",
    "stateBucket",
];

#[derive(Clone, Debug, Default)]
pub struct RealtimeCurrentUserRuntime {
    state: Arc<Mutex<RealtimeCurrentUserState>>,
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

    pub fn apply_ws_message(
        &self,
        generation: u64,
        payload: &RealtimeWsMessagePayload,
        authority: RealtimeCurrentUserAuthority,
    ) -> Option<RealtimeCurrentUserOutput> {
        let message_type = payload.json.get("type").and_then(Value::as_str)?;
        if !matches!(message_type, "user-update" | "user-location") {
            return None;
        }
        let content = payload.json.get("content").unwrap_or(&Value::Null);
        let now = EventTime::from_received_at(&payload.received_at);
        let mut state = self.lock_state();
        if state.generation != generation || state.current_user_id.is_empty() {
            return None;
        }

        match message_type {
            "user-update" => apply_user_update(&mut state, content, &now, &authority),
            "user-location" => apply_user_location(&mut state, content, &now, &authority),
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
        if !authority.local_game_context_available {
            return None;
        }
        let mut state = self.lock_state();
        if state.generation != generation || state.current_user_id.is_empty() {
            return None;
        }
        if authority.is_game_running {
            state.pending_offline = None;
        }
        apply_current_user_patch(
            &mut state,
            Map::new(),
            &EventTime::now(),
            &authority,
            CurrentUserPatchOptions {
                applies_local_game_authority: true,
                reconciles_remote_location: !authority.is_game_running,
                records_current_avatar_history: authority.is_game_running,
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
            || authority.is_game_running
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
        if !authority.local_game_context_available {
            return None;
        }
        let mut state = self.lock_state();
        if state.generation != generation || state.current_user_id.is_empty() {
            return None;
        }
        let previous = state.snapshot.clone();
        let now = EventTime::now();
        let (snapshot, mut persistence) = apply_avatar_wear_transition(
            previous.clone(),
            &previous,
            authority.local_game_context_available,
            false,
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

fn apply_user_update(
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

fn apply_user_location(
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

fn apply_current_user_patch(
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

fn merge_preserved_remote_presence(
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

fn reconcile_remote_game_log_interval(
    state: &mut RealtimeCurrentUserState,
    snapshot: &RealtimeCurrentUserStateSnapshot,
    now: &EventTime,
    game_log_enabled: bool,
    persistence: &mut RealtimePersistenceBatch,
) {
    let location = snapshot.location.trim();
    if !game_log_enabled || !is_real_instance(location) {
        close_remote_game_log_interval(state, now, persistence);
        return;
    }
    if state
        .remote_game_log_interval
        .as_ref()
        .is_some_and(|interval| interval.location == location)
    {
        return;
    }
    close_remote_game_log_interval(state, now, persistence);
    let Some(entry) = location_game_log_entry(snapshot, now) else {
        return;
    };
    state.remote_game_log_interval = Some(RemoteGameLogInterval {
        created_at: entry.created_at.clone(),
        started_at_ms: now.timestamp_ms,
        location: entry.location.clone(),
    });
    persistence.game_log_locations.push(entry);
}

fn close_remote_game_log_interval(
    state: &mut RealtimeCurrentUserState,
    now: &EventTime,
    persistence: &mut RealtimePersistenceBatch,
) {
    let Some(interval) = state.remote_game_log_interval.take() else {
        return;
    };
    persistence
        .game_log_location_time_updates
        .push(GameLogLocationTimeUpdate {
            created_at: interval.created_at,
            time: now.timestamp_ms.saturating_sub(interval.started_at_ms),
        });
}

fn game_log_authority_patch(
    authority: &RealtimeCurrentUserAuthority,
) -> Option<Map<String, Value>> {
    if !authority.local_game_context_available
        || !authority.is_game_running
        || !authority.game_log_enabled
    {
        return None;
    }
    let game_log_location = authority.game_log_location.trim();
    let game_log_destination = authority.game_log_destination.trim();
    let (location, traveling_to_location) = if game_log_location.eq_ignore_ascii_case("traveling")
        && is_real_instance(game_log_destination)
    {
        ("traveling", game_log_destination)
    } else if is_real_instance(game_log_location) {
        (game_log_location, "")
    } else {
        return None;
    };
    let parsed = parse_location(location);
    let parsed_traveling = parse_location(traveling_to_location);
    let world_id = first_owned([parsed.world_id.clone(), parsed_traveling.world_id.clone()]);
    let mut patch = Map::new();
    patch.insert("location".into(), Value::String(location.to_string()));
    patch.insert("worldId".into(), Value::String(world_id));
    patch.insert(
        "instanceId".into(),
        Value::String(parsed.instance_id.clone()),
    );
    patch.insert(
        "travelingToLocation".into(),
        Value::String(traveling_to_location.to_string()),
    );
    patch.insert(
        "travelingToWorld".into(),
        Value::String(parsed_traveling.world_id.clone()),
    );
    patch.insert(
        "travelingToInstance".into(),
        Value::String(parsed_traveling.instance_id.clone()),
    );
    patch.insert("$location".into(), parsed.to_frontend_value(location));
    patch.insert(
        "$travelingToLocation".into(),
        parsed_traveling.to_frontend_value(traveling_to_location),
    );
    let world_name = authority.game_log_world_name.trim();
    if !world_name.is_empty() {
        patch.insert("worldName".into(), Value::String(world_name.to_string()));
    }
    Some(patch)
}

fn apply_avatar_wear_transition(
    mut next: RealtimeCurrentUserStateSnapshot,
    previous: &RealtimeCurrentUserStateSnapshot,
    local_game_context_available: bool,
    is_game_running: bool,
    now: &EventTime,
    records_current_avatar_history: bool,
) -> (RealtimeCurrentUserStateSnapshot, RealtimePersistenceBatch) {
    let previous_avatar_id = previous.current_avatar.clone();
    let next_avatar_id = next.current_avatar.clone();
    let previous_swap_time = previous.previous_avatar_swap_time;
    let mut persistence = RealtimePersistenceBatch::default();

    if !local_game_context_available {
        next.previous_avatar_swap_time = previous_swap_time;
        match previous.raw.get("$previousAvatarSwapTime").cloned() {
            Some(value) => {
                next.raw.insert("$previousAvatarSwapTime".into(), value);
            }
            None => {
                next.raw.remove("$previousAvatarSwapTime");
            }
        }
        return (next, persistence);
    }

    if !is_game_running {
        if !previous_avatar_id.is_empty() && previous_swap_time > 0 {
            persistence
                .avatar_time_spent_upserts
                .push(AvatarTimeSpentUpsert {
                    avatar_id: previous_avatar_id,
                    created_at: now.iso.clone(),
                    time_spent: now.timestamp_ms.saturating_sub(previous_swap_time),
                });
        }
        next.set_previous_avatar_swap_time(None);
        return (next, persistence);
    }
    if next_avatar_id.is_empty() {
        next.set_previous_avatar_swap_time((previous_swap_time > 0).then_some(previous_swap_time));
        return (next, persistence);
    }
    if previous_avatar_id.is_empty() {
        let swap_time = first_positive([next.previous_avatar_swap_time, now.timestamp_ms]);
        next.set_previous_avatar_swap_time(Some(swap_time));
        persistence
            .avatar_history_upserts
            .push(AvatarHistoryUpsert {
                avatar_id: next_avatar_id,
                created_at: now.iso.clone(),
            });
        return (next, persistence);
    }
    if previous_avatar_id != next_avatar_id {
        next.set_previous_avatar_swap_time(Some(now.timestamp_ms));
        persistence
            .avatar_history_upserts
            .push(AvatarHistoryUpsert {
                avatar_id: next_avatar_id,
                created_at: now.iso.clone(),
            });
        if previous_swap_time > 0 {
            persistence
                .avatar_time_spent_upserts
                .push(AvatarTimeSpentUpsert {
                    avatar_id: previous_avatar_id,
                    created_at: now.iso.clone(),
                    time_spent: now.timestamp_ms.saturating_sub(previous_swap_time),
                });
        }
        return (next, persistence);
    }
    let next_swap_time = next.previous_avatar_swap_time;
    if records_current_avatar_history || (previous_swap_time <= 0 && next_swap_time <= 0) {
        persistence
            .avatar_history_upserts
            .push(AvatarHistoryUpsert {
                avatar_id: next_avatar_id,
                created_at: now.iso.clone(),
            });
    }
    next.set_previous_avatar_swap_time(Some(first_positive([
        previous_swap_time,
        next_swap_time,
        now.timestamp_ms,
    ])));
    (next, persistence)
}

fn build_location_patch(
    location: Option<&Value>,
    traveling_to_location: Option<&Value>,
    world_id: Option<&Value>,
) -> Map<String, Value> {
    let location = text_of(location);
    let traveling = text_of(traveling_to_location);
    let parsed_location = parse_location(&location);
    let parsed_traveling = parse_location(&traveling);
    let mut patch = Map::new();
    patch.insert("location".into(), Value::String(location.clone()));
    patch.insert(
        "worldId".into(),
        Value::String(first_owned([
            text_of(world_id),
            parsed_location.world_id.clone(),
        ])),
    );
    patch.insert(
        "instanceId".into(),
        Value::String(parsed_location.instance_id.clone()),
    );
    patch.insert(
        "travelingToLocation".into(),
        Value::String(traveling.clone()),
    );
    patch.insert(
        "travelingToWorld".into(),
        Value::String(parsed_traveling.world_id.clone()),
    );
    patch.insert(
        "travelingToInstance".into(),
        Value::String(parsed_traveling.instance_id.clone()),
    );
    patch.insert(
        "$location".into(),
        parsed_location.to_frontend_value(&location),
    );
    patch.insert(
        "$travelingToLocation".into(),
        parsed_traveling.to_frontend_value(&traveling),
    );
    patch
}

fn location_game_log_entry(
    snapshot: &RealtimeCurrentUserStateSnapshot,
    now: &EventTime,
) -> Option<GameLogLocationEntry> {
    let location = snapshot.location.clone();
    if !is_real_instance(&location) {
        return None;
    }
    let parsed = parse_location(&location);
    let world_name = snapshot.world_name.trim().to_string();
    Some(GameLogLocationEntry {
        created_at: now.iso.clone(),
        location,
        world_id: parsed.world_id,
        world_name,
        time: 0,
        group_name: parsed.group_id.unwrap_or_default(),
    })
}

fn location_game_state_patch(
    snapshot: &RealtimeCurrentUserStateSnapshot,
    now: &EventTime,
) -> Map<String, Value> {
    let location = snapshot.location.clone();
    if !is_real_instance(&location) {
        return map_from_json(json!({
            "currentLocation": "",
            "currentWorldId": "",
            "currentWorldName": "",
            "currentDestination": "",
            "currentLocationStartedAt": null,
            "currentLocationPlayerIds": [],
            "currentLocationPlayers": [],
        }));
    }
    let parsed = parse_location(&location);
    let world_name = snapshot.world_name.trim().to_string();
    map_from_json(json!({
        "currentLocation": location,
        "currentWorldId": parsed.world_id,
        "currentWorldName": world_name,
        "currentDestination": "",
        "currentLocationStartedAt": now.iso,
        "currentLocationPlayerIds": [],
        "currentLocationPlayers": [],
        "lastGameLogAt": now.iso,
        "lastGameLogType": "location",
    }))
}

fn resolve_state_bucket(
    content: &Value,
    patch: &Map<String, Value>,
    previous: Option<&Map<String, Value>>,
) -> Option<String> {
    for value in [
        content.text_field("state"),
        content.text_field("stateBucket"),
        patch.text_field("state"),
        patch.text_field("stateBucket"),
        previous
            .map(|previous| previous.text_field("stateBucket"))
            .unwrap_or_default(),
        previous
            .map(|previous| previous.text_field("state"))
            .unwrap_or_default(),
    ] {
        match value.trim().to_ascii_lowercase().as_str() {
            "online" => return Some("online".into()),
            "active" => return Some("active".into()),
            "offline" => return Some("offline".into()),
            _ => {}
        }
    }
    None
}

fn map_from_json(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn normalize_id(value: &str) -> String {
    value.trim().to_string()
}

fn first_positive(values: impl IntoIterator<Item = i64>) -> i64 {
    values.into_iter().find(|value| *value > 0).unwrap_or(0)
}

fn is_real_instance(location: &str) -> bool {
    let location = location.trim().to_ascii_lowercase();
    if location.is_empty() || location.starts_with("local") {
        return false;
    }
    !matches!(
        location.as_str(),
        ":" | "offline"
            | "offline:offline"
            | "traveling"
            | "traveling:traveling"
            | "private"
            | "private:private"
    )
}

fn is_offline_location(location: &str) -> bool {
    matches!(
        location.trim().to_ascii_lowercase().as_str(),
        "offline" | "offline:offline"
    )
}

fn has_remote_current_user_presence(snapshot: &RealtimeCurrentUserStateSnapshot) -> bool {
    let location = snapshot.location.trim().to_ascii_lowercase();
    !location.is_empty()
        && !location.starts_with("local")
        && !matches!(location.as_str(), ":" | "offline" | "offline:offline")
}

struct EventTime {
    iso: String,
    timestamp_ms: i64,
}

impl EventTime {
    fn now() -> Self {
        let now = Utc::now();
        Self {
            iso: now.to_rfc3339(),
            timestamp_ms: now.timestamp_millis(),
        }
    }

    fn from_received_at(received_at: &str) -> Self {
        let timestamp_ms = DateTime::parse_from_rfc3339(received_at)
            .map(|value| value.timestamp_millis())
            .unwrap_or_else(|_| Utc::now().timestamp_millis());
        Self {
            iso: received_at.to_string(),
            timestamp_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote_authority(game_log_enabled: bool) -> RealtimeCurrentUserAuthority {
        RealtimeCurrentUserAuthority {
            local_game_context_available: true,
            is_game_running: false,
            game_log_enabled,
            ..RealtimeCurrentUserAuthority::default()
        }
    }

    fn current_user_location_message(
        location: &str,
        traveling_to_location: &str,
        received_at: &str,
    ) -> RealtimeWsMessagePayload {
        RealtimeWsMessagePayload {
            json: json!({
                "type": "user-location",
                "content": {
                    "userId": "usr_self",
                    "location": location,
                    "travelingToLocation": traveling_to_location
                }
            }),
            raw: String::new(),
            received_at: received_at.into(),
        }
    }

    #[test]
    fn current_user_projection_serializes_object_shape() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot(
            "usr_self".into(),
            7,
            json!({
                "id": "usr_self",
                "displayName": "Self",
                "location": "offline"
            }),
        );

        let output = runtime
            .apply_ws_message(
                7,
                &RealtimeWsMessagePayload {
                    json: json!({
                        "type": "user-location",
                        "content": {
                            "userId": "usr_self",
                            "location": "wrld_1:123~group(grp_1)",
                            "travelingToLocation": "",
                            "worldId": "wrld_1"
                        }
                    }),
                    raw: String::new(),
                    received_at: "2026-05-15T00:00:00Z".into(),
                },
                RealtimeCurrentUserAuthority::default(),
            )
            .expect("current user location output");

        let serialized = serde_json::to_value(&output.projection).unwrap();
        assert_eq!(serialized["patch"]["id"], json!("usr_self"));
        assert_eq!(
            serialized["snapshot"]["location"],
            json!("wrld_1:123~group(grp_1)")
        );
        assert_eq!(
            serialized["gameStatePatch"]["currentLocation"],
            json!("wrld_1:123~group(grp_1)")
        );
        assert_eq!(
            serialized["patch"]["$location"]["tag"],
            json!("wrld_1:123~group(grp_1)")
        );
        assert_eq!(serialized["patch"]["$location"]["worldId"], json!("wrld_1"));
        assert_eq!(
            serialized["patch"]["$location"]["accessType"],
            json!("group")
        );
        assert_eq!(serialized["patch"]["$location"]["groupId"], json!("grp_1"));
        assert_eq!(
            serialized["patch"]["$travelingToLocation"]["isRealInstance"],
            json!(false)
        );
    }

    #[test]
    fn refreshed_current_user_snapshot_preserves_local_authority_fields() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot(
            "usr_self".into(),
            7,
            json!({
                "id": "usr_self",
                "displayName": "Self",
                "location": "wrld_local:1",
                "worldId": "wrld_local",
                "instanceId": "1",
                "state": "online",
                "stateBucket": "online",
                "status": "join me",
                "statusDescription": "Local status",
                "worldName": "Local World",
                "bio": "old bio"
            }),
        );

        let output = runtime
            .apply_refreshed_snapshot(
                7,
                json!({
                    "id": "usr_self",
                    "displayName": "Self Fresh",
                    "location": "offline",
                    "worldId": "offline",
                    "instanceId": "offline",
                    "state": "offline",
                    "stateBucket": "offline",
                    "status": "busy",
                    "statusDescription": "REST status",
                    "worldName": "REST World",
                    "bio": "fresh bio"
                }),
                json!({}),
                RealtimeCurrentUserAuthority {
                    is_game_running: true,
                    game_log_enabled: true,
                    game_log_location: "wrld_auth:123".into(),
                    game_log_world_name: "Authoritative World".into(),
                    ..RealtimeCurrentUserAuthority::default()
                },
            )
            .expect("refreshed snapshot should update profile fields");

        assert_eq!(
            output.projection.snapshot["displayName"],
            json!("Self Fresh")
        );
        assert_eq!(output.projection.snapshot["bio"], json!("fresh bio"));
        assert_eq!(output.projection.snapshot["status"], json!("join me"));
        assert_eq!(
            output.projection.snapshot["statusDescription"],
            json!("Local status")
        );
        assert_eq!(output.projection.snapshot["stateBucket"], json!("online"));
        assert_eq!(
            output.projection.snapshot["location"],
            json!("wrld_auth:123")
        );
        assert_eq!(output.projection.snapshot["worldId"], json!("wrld_auth"));
        assert_eq!(output.projection.snapshot["instanceId"], json!("123"));
        assert_eq!(
            output.projection.snapshot["worldName"],
            json!("Authoritative World")
        );
        assert_eq!(output.projection.patch["location"], json!("wrld_auth:123"));
        assert_eq!(
            output.projection.patch["$location"]["tag"],
            json!("wrld_auth:123")
        );
    }

    #[test]
    fn refreshed_snapshot_with_stale_sequence_is_dropped() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot(
            "usr_self".into(),
            7,
            json!({ "id": "usr_self", "bio": "old bio" }),
        );
        let stale_sequence = runtime.snapshot_sequence(7).expect("sequence");

        runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
                remote_authority(true),
            )
            .expect("interleaved location apply");

        assert!(runtime
            .apply_refreshed_snapshot_if_sequence(
                7,
                stale_sequence,
                json!({ "id": "usr_self", "bio": "stale bio" }),
                json!({}),
                &[],
                remote_authority(true),
            )
            .is_none());
        let fresh_sequence = runtime.snapshot_sequence(7).expect("sequence");
        let output = runtime
            .apply_refreshed_snapshot_if_sequence(
                7,
                fresh_sequence,
                json!({ "id": "usr_self", "bio": "fresh bio" }),
                json!({}),
                &[],
                remote_authority(true),
            )
            .expect("fresh sequence applies");
        assert_eq!(output.projection.snapshot["bio"], json!("fresh bio"));
    }

    #[test]
    fn interleaved_avatar_and_fallback_selection_drops_the_stale_response() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot(
            "usr_self".into(),
            7,
            json!({
                "id": "usr_self",
                "currentAvatar": "avtr_old",
                "fallbackAvatar": "avtr_old_fallback"
            }),
        );
        let shared_sequence = runtime.snapshot_sequence(7).expect("sequence");

        let avatar_output = runtime
            .apply_refreshed_snapshot_if_sequence(
                7,
                shared_sequence,
                json!({
                    "id": "usr_self",
                    "currentAvatar": "avtr_new",
                    "fallbackAvatar": "avtr_old_fallback"
                }),
                json!({}),
                CURRENT_USER_AVATAR_RESPONSE_AUTHORITY_FIELDS,
                remote_authority(true),
            )
            .expect("avatar selection response applies");
        assert_eq!(
            avatar_output.projection.snapshot["currentAvatar"],
            json!("avtr_new")
        );

        assert!(runtime
            .apply_refreshed_snapshot_if_sequence(
                7,
                shared_sequence,
                json!({
                    "id": "usr_self",
                    "currentAvatar": "avtr_old",
                    "fallbackAvatar": "avtr_new_fallback"
                }),
                json!({}),
                CURRENT_USER_FALLBACK_AVATAR_RESPONSE_AUTHORITY_FIELDS,
                remote_authority(true),
            )
            .is_none());
        let snapshot = runtime.snapshot_value().expect("snapshot");
        assert_eq!(snapshot["currentAvatar"], json!("avtr_new"));
        assert_eq!(snapshot["fallbackAvatar"], json!("avtr_old_fallback"));
    }

    #[test]
    fn response_authority_fields_override_the_local_authority_strip() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot(
            "usr_self".into(),
            7,
            json!({ "id": "usr_self", "status": "join me" }),
        );
        let sequence = runtime.snapshot_sequence(7).expect("sequence");

        let output = runtime
            .apply_refreshed_snapshot_if_sequence(
                7,
                sequence,
                json!({ "id": "usr_self", "status": "busy" }),
                json!({}),
                &["status"],
                remote_authority(true),
            )
            .expect("response authority field applies");

        assert_eq!(output.projection.snapshot["status"], json!("busy"));
    }

    #[test]
    fn unavailable_local_game_context_skips_game_dependent_side_effects() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot(
            "usr_self".into(),
            7,
            json!({
                "id": "usr_self",
                "currentAvatar": "avtr_current",
                "$previousAvatarSwapTime": 1_000
            }),
        );
        let authority = RealtimeCurrentUserAuthority {
            local_game_context_available: false,
            ..RealtimeCurrentUserAuthority::default()
        };

        let output = runtime
            .apply_ws_message(
                7,
                &RealtimeWsMessagePayload {
                    json: json!({
                        "type": "user-location",
                        "content": {
                            "userId": "usr_self",
                            "location": "wrld_1:123",
                            "travelingToLocation": "",
                            "worldId": "wrld_1"
                        }
                    }),
                    raw: String::new(),
                    received_at: "2026-05-15T00:00:02Z".into(),
                },
                authority.clone(),
            )
            .expect("current user location output");

        assert_eq!(output.projection.snapshot["location"], json!("wrld_1:123"));
        assert_eq!(
            output.projection.snapshot["$previousAvatarSwapTime"],
            json!(1_000)
        );
        assert!(output.projection.game_state_patch.is_none());
        assert!(output.persistence.is_empty());
        assert!(runtime.apply_game_running_state(7, authority).is_none());
    }

    #[test]
    fn running_local_game_keeps_authoritative_location_above_remote_ws_location() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot(
            "usr_self".into(),
            7,
            json!({
                "id": "usr_self",
                "location": "wrld_local:123",
                "worldId": "wrld_local",
                "instanceId": "123",
                "state": "online",
                "stateBucket": "online"
            }),
        );

        let output = runtime
            .apply_ws_message(
                7,
                &RealtimeWsMessagePayload {
                    json: json!({
                        "type": "user-location",
                        "content": {
                            "userId": "usr_self",
                            "location": "wrld_remote:456",
                            "travelingToLocation": "",
                            "worldId": "wrld_remote"
                        }
                    }),
                    raw: String::new(),
                    received_at: "2026-05-15T00:00:00Z".into(),
                },
                RealtimeCurrentUserAuthority {
                    is_game_running: true,
                    game_log_enabled: true,
                    game_log_location: "wrld_local:123".into(),
                    game_log_world_name: "Local World".into(),
                    ..RealtimeCurrentUserAuthority::default()
                },
            )
            .expect("current user location output");

        assert_eq!(
            output.projection.snapshot["location"],
            json!("wrld_local:123")
        );
        assert_eq!(output.projection.snapshot["worldId"], json!("wrld_local"));
        assert!(output.projection.game_state_patch.is_none());
        assert!(output.persistence.game_log_locations.is_empty());
    }

    #[test]
    fn stopped_local_game_projects_remote_location_as_online_and_starts_gamelog_interval() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot(
            "usr_self".into(),
            7,
            json!({
                "id": "usr_self",
                "status": "busy",
                "location": "offline",
                "state": "offline",
                "stateBucket": "offline"
            }),
        );

        let output = runtime
            .apply_ws_message(
                7,
                &current_user_location_message(
                    "wrld_remote:456~group(grp_remote)",
                    "",
                    "2026-05-15T00:00:00Z",
                ),
                remote_authority(true),
            )
            .expect("remote location output");

        assert_eq!(output.projection.snapshot["state"], json!("online"));
        assert_eq!(output.projection.snapshot["stateBucket"], json!("online"));
        assert_eq!(
            output.projection.snapshot["location"],
            json!("wrld_remote:456~group(grp_remote)")
        );
        assert!(output.projection.snapshot.get("pendingOffline").is_none());
        assert_eq!(output.persistence.game_log_locations.len(), 1);
        assert_eq!(
            output.persistence.game_log_locations[0],
            GameLogLocationEntry {
                created_at: "2026-05-15T00:00:00Z".into(),
                location: "wrld_remote:456~group(grp_remote)".into(),
                world_id: "wrld_remote".into(),
                world_name: "".into(),
                time: 0,
                group_name: "grp_remote".into(),
            }
        );
        assert_eq!(output.timer_action, PendingOfflineTimerAction::None);
    }

    #[test]
    fn false_remote_offline_keeps_location_until_same_location_cancels_pending() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
        runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
                remote_authority(true),
            )
            .expect("remote interval start");

        let pending = runtime
            .apply_ws_message(
                7,
                &current_user_location_message("offline:offline", "", "2026-05-15T00:00:10Z"),
                remote_authority(true),
            )
            .expect("remote offline pending output");
        let PendingOfflineTimerAction::Schedule {
            user_id,
            token,
            delay_ms,
        } = pending.timer_action
        else {
            panic!("remote offline should schedule pending timer");
        };

        assert_eq!(user_id, "usr_self");
        assert_eq!(delay_ms, 170_000);
        assert_eq!(
            pending.projection.snapshot["location"],
            json!("wrld_remote:456")
        );
        assert_eq!(pending.projection.snapshot["stateBucket"], json!("online"));
        assert!(pending.persistence.is_empty());

        let resumed = runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:10.004Z"),
                remote_authority(true),
            )
            .expect("same remote location should cancel pending");

        assert_eq!(resumed.timer_action, PendingOfflineTimerAction::None);
        assert!(resumed.persistence.is_empty());
        assert!(runtime
            .fire_pending_offline(
                7,
                token,
                "2026-05-15T00:03:00Z".into(),
                remote_authority(true),
            )
            .is_none());
    }

    #[test]
    fn confirmed_remote_offline_ends_interval_and_same_location_can_start_again() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
        runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
                remote_authority(true),
            )
            .expect("remote interval start");
        let pending = runtime
            .apply_ws_message(
                7,
                &current_user_location_message("offline", "", "2026-05-15T00:00:10Z"),
                remote_authority(true),
            )
            .expect("remote offline pending output");
        let PendingOfflineTimerAction::Schedule { token, .. } = pending.timer_action else {
            panic!("remote offline should schedule pending timer");
        };

        let confirmed = runtime
            .fire_pending_offline(
                7,
                token,
                "2026-05-15T00:03:00Z".into(),
                remote_authority(true),
            )
            .expect("pending remote offline should fire");

        assert_eq!(confirmed.projection.snapshot["state"], json!("active"));
        assert_eq!(
            confirmed.projection.snapshot["stateBucket"],
            json!("active")
        );
        assert_eq!(confirmed.projection.snapshot["location"], json!("offline"));
        assert_eq!(
            confirmed.persistence.game_log_location_time_updates,
            vec![GameLogLocationTimeUpdate {
                created_at: "2026-05-15T00:00:00Z".into(),
                time: 180_000,
            }]
        );

        let restarted = runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:03:20Z"),
                remote_authority(true),
            )
            .expect("same location after confirmed offline starts a new interval");
        assert_eq!(restarted.persistence.game_log_locations.len(), 1);
        assert_eq!(
            restarted.persistence.game_log_locations[0].created_at,
            "2026-05-15T00:03:20Z"
        );
    }

    #[test]
    fn remote_presence_remains_visible_when_gamelog_is_disabled_without_writes() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));

        let output = runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
                remote_authority(false),
            )
            .expect("remote presence output");

        assert_eq!(output.projection.snapshot["stateBucket"], json!("online"));
        assert!(output.persistence.is_empty());
    }

    #[test]
    fn local_game_start_invalidates_remote_offline_timer_and_keeps_local_authority() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
        runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
                remote_authority(true),
            )
            .expect("remote interval start");
        let pending = runtime
            .apply_ws_message(
                7,
                &current_user_location_message("offline", "", "2026-05-15T00:00:10Z"),
                remote_authority(true),
            )
            .expect("remote offline pending output");
        let PendingOfflineTimerAction::Schedule { token, .. } = pending.timer_action else {
            panic!("remote offline should schedule pending timer");
        };
        let local_authority = RealtimeCurrentUserAuthority {
            local_game_context_available: true,
            is_game_running: true,
            game_log_enabled: true,
            game_log_location: "wrld_local:123".into(),
            game_log_world_name: "Local World".into(),
            ..RealtimeCurrentUserAuthority::default()
        };

        let local = runtime
            .apply_game_running_state(7, local_authority.clone())
            .expect("local game state output");

        assert_eq!(
            local.projection.snapshot["location"],
            json!("wrld_local:123")
        );
        assert_eq!(local.projection.snapshot["stateBucket"], json!("online"));
        assert!(runtime
            .fire_pending_offline(7, token, "2026-05-15T00:03:00Z".into(), local_authority,)
            .is_none());
    }

    #[test]
    fn reconnect_preserves_remote_interval_and_invalidates_old_pending_timer() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
        runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
                remote_authority(true),
            )
            .expect("remote interval start");
        let pending = runtime
            .apply_ws_message(
                7,
                &current_user_location_message("offline", "", "2026-05-15T00:00:10Z"),
                remote_authority(true),
            )
            .expect("remote offline pending output");
        let PendingOfflineTimerAction::Schedule {
            token: old_token, ..
        } = pending.timer_action
        else {
            panic!("remote offline should schedule pending timer");
        };

        runtime.set_snapshot(
            "usr_self".into(),
            8,
            json!({
                "id": "usr_self",
                "location": "wrld_remote:456",
                "state": "online",
                "stateBucket": "online"
            }),
        );

        assert!(runtime
            .fire_pending_offline(
                7,
                old_token,
                "2026-05-15T00:03:00Z".into(),
                remote_authority(true),
            )
            .is_none());
        let duplicate = runtime
            .apply_ws_message(
                8,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:20Z"),
                remote_authority(true),
            )
            .expect("reconnected remote location output");
        assert!(duplicate.persistence.game_log_locations.is_empty());

        let pending = runtime
            .apply_ws_message(
                8,
                &current_user_location_message("offline", "", "2026-05-15T00:00:30Z"),
                remote_authority(true),
            )
            .expect("remote offline after reconnect");
        let PendingOfflineTimerAction::Schedule { token, .. } = pending.timer_action else {
            panic!("remote offline should schedule pending timer");
        };
        let confirmed = runtime
            .fire_pending_offline(
                8,
                token,
                "2026-05-15T00:03:20Z".into(),
                remote_authority(true),
            )
            .expect("remote offline should close original interval");

        assert_eq!(
            confirmed.persistence.game_log_location_time_updates,
            vec![GameLogLocationTimeUpdate {
                created_at: "2026-05-15T00:00:00Z".into(),
                time: 200_000,
            }]
        );
    }

    #[test]
    fn transport_interruption_does_not_end_remote_interval_or_change_presence() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
        runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
                remote_authority(true),
            )
            .expect("remote interval start");

        let finalized = runtime
            .interrupt_transport(7, remote_authority(true))
            .expect("transport finalization output");

        assert_eq!(
            finalized.projection.snapshot["location"],
            json!("wrld_remote:456")
        );
        assert_eq!(
            finalized.projection.snapshot["stateBucket"],
            json!("online")
        );
        assert!(finalized
            .persistence
            .game_log_location_time_updates
            .is_empty());
    }

    #[test]
    fn explicit_transport_finalization_ends_remote_interval() {
        let runtime = RealtimeCurrentUserRuntime::new();
        runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
        runtime
            .apply_ws_message(
                7,
                &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
                remote_authority(true),
            )
            .expect("remote interval start");

        let finalized = runtime
            .finalize_transport(7, remote_authority(true))
            .expect("explicit transport finalization output");

        assert_eq!(
            finalized.persistence.game_log_location_time_updates.len(),
            1
        );
        assert_eq!(
            finalized.persistence.game_log_location_time_updates[0].created_at,
            "2026-05-15T00:00:00Z"
        );
        assert!(finalized.persistence.game_log_location_time_updates[0].time > 0);
    }
}
