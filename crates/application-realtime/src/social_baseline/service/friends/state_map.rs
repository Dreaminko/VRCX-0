use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::super::{object_field, string_array_field, unique_push};

pub(in super::super) struct FriendStateMap {
    pub(in super::super) state_by_id: HashMap<String, String>,
    pub(in super::super) ordered_ids: Vec<String>,
}

pub(in super::super) struct SnapshotFriendIds {
    pub(in super::super) friend_ids: Vec<String>,
    pub(in super::super) has_friend_list: bool,
}

fn add_state_bucket_ids(
    snapshot: &Value,
    key: &str,
    state_bucket: &str,
    state_by_id: &mut HashMap<String, String>,
    ordered_ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    for user_id in string_array_field(snapshot, key) {
        if user_id.is_empty() {
            continue;
        }
        unique_push(ordered_ids, seen, user_id.clone());
        state_by_id.insert(user_id, state_bucket.to_string());
    }
}

pub(in super::super) fn build_friend_state_map(snapshot: &Value) -> FriendStateMap {
    let mut state_by_id = HashMap::new();
    let mut ordered_ids = Vec::new();
    let mut seen = HashSet::new();
    add_state_bucket_ids(
        snapshot,
        "friends",
        "offline",
        &mut state_by_id,
        &mut ordered_ids,
        &mut seen,
    );
    add_state_bucket_ids(
        snapshot,
        "offlineFriends",
        "offline",
        &mut state_by_id,
        &mut ordered_ids,
        &mut seen,
    );
    add_state_bucket_ids(
        snapshot,
        "activeFriends",
        "active",
        &mut state_by_id,
        &mut ordered_ids,
        &mut seen,
    );
    add_state_bucket_ids(
        snapshot,
        "onlineFriends",
        "online",
        &mut state_by_id,
        &mut ordered_ids,
        &mut seen,
    );
    FriendStateMap {
        state_by_id,
        ordered_ids,
    }
}

pub(in super::super) fn build_snapshot_friend_ids(snapshot: &Value) -> SnapshotFriendIds {
    let has_friend_list = object_field(snapshot, "friends").is_some_and(Value::is_array);
    let friend_ids = string_array_field(snapshot, "friends");
    SnapshotFriendIds {
        friend_ids,
        has_friend_list,
    }
}
