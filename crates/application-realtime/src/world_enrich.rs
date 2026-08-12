use serde_json::Value;
pub(crate) use vrcx_0_core::location::is_meaningful_world_name;
use vrcx_0_core::location::{format_display_location, parse_location, world_id_from_location};

use vrcx_0_application_core::{RealtimeEntryCorrectionStream, WorldCache};
use vrcx_0_core::text::{first_non_empty, first_non_empty_owned};

#[derive(Clone, Debug)]
pub(crate) struct PendingWorldNameResolution {
    pub(crate) world_id: String,
    pub(crate) entry: Option<PendingEntryCorrection>,
}

impl PendingWorldNameResolution {
    pub(crate) fn cache_only(world_id: String) -> Self {
        Self {
            world_id,
            entry: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PendingEntryCorrection {
    pub(crate) stream: RealtimeEntryCorrectionStream,
    pub(crate) id: String,
    pub(crate) location: String,
    pub(crate) group_name: String,
}

pub(crate) fn enrich_world_name(
    world_cache: &WorldCache,
    value: &mut Value,
    stream: Option<RealtimeEntryCorrectionStream>,
) -> Option<PendingWorldNameResolution> {
    let object = value.as_object_mut()?;
    let top_level_name = object_string(object, "worldName");
    let details_name = nested_object_string(object, &["details", "worldName"]);
    let top_level_is_meaningful = is_meaningful_world_name(&top_level_name);
    let details_is_meaningful = is_meaningful_world_name(&details_name);

    let mut unresolved_world_id = None;
    let world_id = notification_world_id_from_object(object);
    let world_name = if top_level_is_meaningful {
        Some(top_level_name)
    } else if details_is_meaningful {
        Some(details_name)
    } else if world_id.is_empty() {
        None
    } else {
        match world_cache.get_name(&world_id) {
            Some(world_name) => Some(world_name),
            None => {
                unresolved_world_id = Some(world_id.clone());
                None
            }
        }
    };

    if let Some(world_name) = world_name {
        if !top_level_is_meaningful {
            object.insert("worldName".into(), Value::String(world_name.clone()));
        }
        if !details_is_meaningful {
            if let Some(details) = object.get_mut("details").and_then(Value::as_object_mut) {
                details.insert("worldName".into(), Value::String(world_name));
            }
        }
        if !world_id.is_empty() && object_str(object, "worldId").is_empty() {
            object.insert("worldId".into(), Value::String(world_id));
        }
    }
    apply_display_location(object);
    unresolved_world_id.map(|world_id| PendingWorldNameResolution {
        world_id,
        entry: stream.and_then(|stream| pending_entry_correction(object, stream)),
    })
}

pub(crate) fn resolved_display_location(
    location: &str,
    world_name: &str,
    group_name: &str,
) -> String {
    let parsed = parse_location(location);
    format_display_location(&parsed, world_name, group_name)
}

pub(crate) fn feed_entry_correction_id(object: &serde_json::Map<String, Value>) -> String {
    let id = object_str(object, "id");
    if !id.is_empty() {
        return format!("id:{id}");
    }
    let row_id = first_non_empty([object_str(object, "rowId"), object_str(object, "row_id")]);
    if !row_id.is_empty() {
        let source_rank = first_non_empty([
            object_str(object, "sourceRank"),
            object_str(object, "source_rank"),
        ]);
        let entry_type = object_str(object, "type");
        if !source_rank.is_empty() {
            return format!("row:{entry_type}:{source_rank}:{row_id}");
        }
        return format!("row:{entry_type}:{row_id}");
    }
    let entry_type = object_str(object, "type");
    let created_at = first_non_empty([
        object_str(object, "created_at"),
        object_str(object, "createdAt"),
    ]);
    let user_id = first_non_empty([
        object_str(object, "userId"),
        object_str(object, "senderUserId"),
    ]);
    let location = first_non_empty([
        object_str(object, "location"),
        nested_object_str(object, &["details", "location"]),
    ]);
    let message = object_str(object, "message");
    format!("{entry_type}:{created_at}:{user_id}:{location}:{message}")
}

fn notification_world_id_from_object(object: &serde_json::Map<String, Value>) -> String {
    first_world_id([
        object_str(object, "worldId"),
        object_str(object, "worldName"),
        object_str(object, "location"),
        object_str(object, "instanceLocation"),
        nested_object_str(object, &["details", "worldId"]),
        nested_object_str(object, &["details", "worldName"]),
        nested_object_str(object, &["details", "location"]),
    ])
}

fn pending_entry_correction(
    object: &serde_json::Map<String, Value>,
    stream: RealtimeEntryCorrectionStream,
) -> Option<PendingEntryCorrection> {
    let id = match stream {
        RealtimeEntryCorrectionStream::Feed => feed_entry_correction_id(object),
        RealtimeEntryCorrectionStream::Notification => notification_id_from_object(object),
    };
    (!id.trim().is_empty()).then(|| PendingEntryCorrection {
        stream,
        id,
        location: first_non_empty_owned([
            object_str(object, "location"),
            nested_object_str(object, &["details", "location"]),
            object_str(object, "instanceLocation"),
        ]),
        group_name: first_non_empty_owned([
            object_str(object, "groupName"),
            nested_object_str(object, &["details", "groupName"]),
        ]),
    })
}

fn notification_id_from_object(object: &serde_json::Map<String, Value>) -> String {
    first_non_empty_owned([
        object_str(object, "id"),
        object_str(object, "notificationId"),
    ])
}

fn apply_display_location(object: &mut serde_json::Map<String, Value>) {
    let location = first_non_empty([
        object_str(object, "location"),
        nested_object_str(object, &["details", "location"]),
        object_str(object, "instanceLocation"),
    ]);
    let world_name = first_non_empty([
        object_str(object, "worldName"),
        nested_object_str(object, &["details", "worldName"]),
    ]);
    let group_name = first_non_empty([
        object_str(object, "groupName"),
        nested_object_str(object, &["details", "groupName"]),
    ]);
    let display_location = resolved_display_location(location, world_name, group_name);
    if !display_location.is_empty() {
        object.insert("displayLocation".into(), Value::String(display_location));
    }
}

fn object_string(object: &serde_json::Map<String, Value>, key: &str) -> String {
    object_str(object, key).to_string()
}

fn object_str<'a>(object: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
}

fn nested_object_string(object: &serde_json::Map<String, Value>, path: &[&str]) -> String {
    nested_object_str(object, path).to_string()
}

fn nested_object_str<'a>(object: &'a serde_json::Map<String, Value>, path: &[&str]) -> &'a str {
    let Some((first, rest)) = path.split_first() else {
        return "";
    };
    let Some(mut current) = object.get(*first) else {
        return "";
    };
    for key in rest {
        let Some(next) = current.get(*key) else {
            return "";
        };
        current = next;
    }
    current.as_str().map(str::trim).unwrap_or_default()
}

fn first_world_id<const N: usize>(values: [&str; N]) -> String {
    values
        .into_iter()
        .map(world_id_from_location_or_id)
        .find(|value| !value.is_empty())
        .unwrap_or_default()
}

pub fn world_id_from_location_or_id(value: &str) -> String {
    world_id_from_location(value)
}
