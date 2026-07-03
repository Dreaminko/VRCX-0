use serde_json::Value;

use crate::common::ParamsBuilder;
use crate::database::{DatabaseService, DatabaseWriteTransaction};
use crate::game_log::{ensure_game_log_tables, GameLogLocationEntry};
use crate::Error;

use super::schema::{ensure_realtime_tables, normalize_user_table_prefix};
use super::types::*;

#[derive(Clone, Debug, Default)]
struct ExistingFriendLogRow {
    user_id: String,
    display_name: String,
    trust_level: String,
    friend_number: i64,
}

struct FriendLogHistoryEntry<'a> {
    created_at: &'a str,
    entry_type: &'a str,
    user_id: &'a str,
    display_name: &'a str,
    previous_display_name: &'a str,
    trust_level: &'a str,
    previous_trust_level: &'a str,
    friend_number: i64,
}

pub fn write_realtime_batch(
    db: &DatabaseService,
    owner_user_id: &str,
    batch: &RealtimePersistenceBatch,
) -> Result<RealtimeWriteCounts, Error> {
    if batch.is_empty() {
        return Ok(RealtimeWriteCounts::default());
    }

    let owner_user_id = normalize_user_id(owner_user_id);
    if owner_user_id.is_empty() {
        return Err(Error::Database(
            "Realtime persistence requires a current user id.".into(),
        ));
    }
    let user_prefix = normalize_user_table_prefix(&owner_user_id)?;
    ensure_realtime_tables(db, &user_prefix)?;
    if !batch.game_log_locations.is_empty() {
        ensure_game_log_tables(db)?;
    }
    db.write_transaction(|tx| {
        let mut counts = RealtimeWriteCounts::default();
        for entry in &batch.friend_log_upserts {
            counts.add_realtime_rows(upsert_friend_log_current(tx, &user_prefix, entry)?);
        }
        for entry in &batch.friend_log_deletes {
            counts.add_realtime_rows(delete_friend_log_current(tx, &user_prefix, entry)?);
        }
        for entry in &batch.feed_entries {
            counts.add_realtime_rows(insert_feed_entry(tx, &user_prefix, entry)?);
        }
        for entry in &batch.notification_v1_upserts {
            counts.add_realtime_rows(upsert_notification_v1(tx, &user_prefix, entry)?);
        }
        for entry in &batch.notification_v2_upserts {
            counts.add_realtime_rows(upsert_notification_v2(tx, &user_prefix, entry)?);
        }
        for entry in &batch.notification_v2_updates {
            counts.add_realtime_rows(update_notification_v2(tx, &user_prefix, entry)?);
        }
        for entry in &batch.notification_expirations {
            counts.add_realtime_rows(expire_notification(tx, &user_prefix, entry)?);
        }
        for id in &batch.notification_seen {
            counts.add_realtime_rows(mark_notification_seen(tx, &user_prefix, id)?);
        }
        for entry in &batch.avatar_history_upserts {
            counts.add_realtime_rows(upsert_avatar_history(tx, &user_prefix, entry)?);
        }
        for entry in &batch.avatar_time_spent_upserts {
            counts.add_realtime_rows(upsert_avatar_time_spent(tx, &user_prefix, entry)?);
        }
        for entry in &batch.game_log_locations {
            counts.add_game_log_rows(insert_game_log_location_if_changed(tx, entry)?);
        }
        Ok(counts)
    })
}

fn upsert_friend_log_current(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    entry: &FriendLogUpsert,
) -> Result<u64, Error> {
    let target_user_id = normalize_user_id(&entry.target_user_id);
    if target_user_id.is_empty() {
        return Ok(0);
    }
    let existing_rows = tx.execute(
        &format!(
            "SELECT user_id, display_name, trust_level, friend_number FROM {user_prefix}_friend_log_current WHERE user_id = @user_id LIMIT 1"
        ),
        &ParamsBuilder::new().set("user_id", target_user_id.clone()).build(),
    )?;
    let existing = existing_rows
        .first()
        .map(|row| existing_friend_log_row(row));
    let friend_number = if entry.friend_number > 0 {
        entry.friend_number
    } else if let Some(existing) = existing.as_ref() {
        existing.friend_number
    } else {
        next_friend_number(tx, user_prefix)?
    };
    let existing_display_name = existing
        .as_ref()
        .map(|existing| existing.display_name.trim())
        .unwrap_or("");
    let entry_display_name = entry.display_name.trim();
    let display_name = if !entry_display_name.is_empty() && entry_display_name != "Unknown" {
        entry_display_name
    } else if !existing_display_name.is_empty() && existing_display_name != "Unknown" {
        existing_display_name
    } else {
        "Unknown"
    };
    let trust_level = first_non_empty([entry.trust_level.as_str(), "Visitor"]);
    let insert_count = tx.execute_non_query(
        &format!(
            "INSERT OR IGNORE INTO {user_prefix}_friend_log_current (user_id, display_name, trust_level, friend_number) VALUES (@user_id, @display_name, @trust_level, @friend_number)"
        ),
        &ParamsBuilder::new()
            .set("user_id", target_user_id.clone())
            .set("display_name", display_name)
            .set("trust_level", trust_level)
            .set("friend_number", friend_number)
            .build(),
    )?;
    let mut affected = affected_count(insert_count);
    if insert_count <= 0 {
        affected = affected.saturating_add(affected_count(tx.execute_non_query(
            &format!(
                "UPDATE {user_prefix}_friend_log_current SET display_name = @display_name, trust_level = @trust_level, friend_number = CASE WHEN @friend_number > 0 THEN @friend_number ELSE friend_number END WHERE user_id = @user_id"
            ),
            &ParamsBuilder::new()
                .set("user_id", target_user_id.clone())
                .set("display_name", display_name)
                .set("trust_level", trust_level)
                .set("friend_number", friend_number)
                .build(),
        )?));
        if entry.force_history {
            affected = affected.saturating_add(add_friend_log_history(
                tx,
                user_prefix,
                &FriendLogHistoryEntry {
                    created_at: &entry.created_at,
                    entry_type: "Friend",
                    user_id: &target_user_id,
                    display_name,
                    previous_display_name: "",
                    trust_level,
                    previous_trust_level: "",
                    friend_number,
                },
            )?);
        }
    } else {
        affected = affected.saturating_add(add_friend_log_history(
            tx,
            user_prefix,
            &FriendLogHistoryEntry {
                created_at: &entry.created_at,
                entry_type: "Friend",
                user_id: &target_user_id,
                display_name,
                previous_display_name: "",
                trust_level,
                previous_trust_level: "",
                friend_number,
            },
        )?);
    }
    Ok(affected)
}

fn delete_friend_log_current(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    entry: &FriendLogDelete,
) -> Result<u64, Error> {
    let target_user_id = normalize_user_id(&entry.target_user_id);
    if target_user_id.is_empty() {
        return Ok(0);
    }
    let existing_rows = tx.execute(
        &format!(
            "SELECT user_id, display_name, trust_level, friend_number FROM {user_prefix}_friend_log_current WHERE user_id = @user_id LIMIT 1"
        ),
        &ParamsBuilder::new().set("user_id", target_user_id.clone()).build(),
    )?;
    let Some(existing) = existing_rows
        .first()
        .map(|row| existing_friend_log_row(row))
    else {
        return Ok(0);
    };
    let deleted = tx.execute_non_query(
        &format!("DELETE FROM {user_prefix}_friend_log_current WHERE user_id = @user_id"),
        &ParamsBuilder::new().set("user_id", target_user_id).build(),
    )?;
    let mut affected = affected_count(deleted);
    if deleted > 0 {
        affected = affected.saturating_add(add_friend_log_history(
            tx,
            user_prefix,
            &FriendLogHistoryEntry {
                created_at: &entry.created_at,
                entry_type: "Unfriend",
                user_id: &existing.user_id,
                display_name: &existing.display_name,
                previous_display_name: "",
                trust_level: &existing.trust_level,
                previous_trust_level: "",
                friend_number: existing.friend_number,
            },
        )?);
    }
    Ok(affected)
}

fn add_friend_log_history(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    entry: &FriendLogHistoryEntry<'_>,
) -> Result<u64, Error> {
    tx.execute_non_query(
        &format!(
            "INSERT INTO {user_prefix}_friend_log_history (created_at, type, user_id, display_name, previous_display_name, trust_level, previous_trust_level, friend_number) VALUES (@created_at, @type, @user_id, @display_name, @previous_display_name, @trust_level, @previous_trust_level, @friend_number)"
        ),
        &ParamsBuilder::new()
            .set("created_at", entry.created_at)
            .set("type", entry.entry_type)
            .set("user_id", entry.user_id)
            .set("display_name", entry.display_name)
            .set("previous_display_name", entry.previous_display_name)
            .set("trust_level", entry.trust_level)
            .set("previous_trust_level", entry.previous_trust_level)
            .set("friend_number", entry.friend_number)
            .build(),
    )
    .map(affected_count)
}

fn insert_feed_entry(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    entry: &Value,
) -> Result<u64, Error> {
    let entry_type = entry_string(entry, "type");
    let affected = match entry_type.as_str() {
        "GPS" => tx.execute_non_query(
            &format!("INSERT OR IGNORE INTO {user_prefix}_feed_gps (created_at, user_id, display_name, location, world_name, previous_location, time, group_name) VALUES (@created_at, @user_id, @display_name, @location, @world_name, @previous_location, @time, @group_name)"),
            &ParamsBuilder::new()
                .set("created_at", entry_string(entry, "created_at"))
                .set("user_id", entry_string(entry, "userId"))
                .set("display_name", entry_string(entry, "displayName"))
                .set("location", entry_string(entry, "location"))
                .set("world_name", entry_string(entry, "worldName"))
                .set("previous_location", entry_string(entry, "previousLocation"))
                .set("time", entry_i64(entry, "time"))
                .set("group_name", entry_string(entry, "groupName"))
                .build(),
        )?,
        "Online" | "Offline" => tx.execute_non_query(
            &format!("INSERT OR IGNORE INTO {user_prefix}_feed_online_offline (created_at, user_id, display_name, type, location, world_name, time, group_name) VALUES (@created_at, @user_id, @display_name, @type, @location, @world_name, @time, @group_name)"),
            &ParamsBuilder::new()
                .set("created_at", entry_string(entry, "created_at"))
                .set("user_id", entry_string(entry, "userId"))
                .set("display_name", entry_string(entry, "displayName"))
                .set("type", entry_type)
                .set("location", entry_string(entry, "location"))
                .set("world_name", entry_string(entry, "worldName"))
                .set("time", entry_i64(entry, "time"))
                .set("group_name", entry_string(entry, "groupName"))
                .build(),
        )?,
        "Status" => tx.execute_non_query(
            &format!("INSERT OR IGNORE INTO {user_prefix}_feed_status (created_at, user_id, display_name, status, status_description, previous_status, previous_status_description) VALUES (@created_at, @user_id, @display_name, @status, @status_description, @previous_status, @previous_status_description)"),
            &ParamsBuilder::new()
                .set("created_at", entry_string(entry, "created_at"))
                .set("user_id", entry_string(entry, "userId"))
                .set("display_name", entry_string(entry, "displayName"))
                .set("status", entry_string(entry, "status"))
                .set("status_description", entry_string(entry, "statusDescription"))
                .set("previous_status", entry_string(entry, "previousStatus"))
                .set("previous_status_description", entry_string(entry, "previousStatusDescription"))
                .build(),
        )?,
        "Bio" => tx.execute_non_query(
            &format!("INSERT OR IGNORE INTO {user_prefix}_feed_bio (created_at, user_id, display_name, bio, previous_bio) VALUES (@created_at, @user_id, @display_name, @bio, @previous_bio)"),
            &ParamsBuilder::new()
                .set("created_at", entry_string(entry, "created_at"))
                .set("user_id", entry_string(entry, "userId"))
                .set("display_name", entry_string(entry, "displayName"))
                .set("bio", entry_string(entry, "bio"))
                .set("previous_bio", entry_string(entry, "previousBio"))
                .build(),
        )?,
        "Avatar" => tx.execute_non_query(
            &format!("INSERT OR IGNORE INTO {user_prefix}_feed_avatar (created_at, user_id, display_name, owner_id, avatar_name, current_avatar_image_url, current_avatar_thumbnail_image_url, previous_current_avatar_image_url, previous_current_avatar_thumbnail_image_url) VALUES (@created_at, @user_id, @display_name, @owner_id, @avatar_name, @current_avatar_image_url, @current_avatar_thumbnail_image_url, @previous_current_avatar_image_url, @previous_current_avatar_thumbnail_image_url)"),
            &ParamsBuilder::new()
                .set("created_at", entry_string(entry, "created_at"))
                .set("user_id", entry_string(entry, "userId"))
                .set("display_name", entry_string(entry, "displayName"))
                .set("owner_id", entry_string(entry, "ownerId"))
                .set("avatar_name", entry_string(entry, "avatarName"))
                .set("current_avatar_image_url", entry_string(entry, "currentAvatarImageUrl"))
                .set("current_avatar_thumbnail_image_url", entry_string(entry, "currentAvatarThumbnailImageUrl"))
                .set("previous_current_avatar_image_url", entry_string(entry, "previousCurrentAvatarImageUrl"))
                .set("previous_current_avatar_thumbnail_image_url", entry_string(entry, "previousCurrentAvatarThumbnailImageUrl"))
                .build(),
        )?,
        other => {
            return Err(Error::InvalidData(format!(
                "Unknown realtime feed entry type: {other}"
            )));
        }
    };
    Ok(affected_count(affected))
}

fn upsert_notification_v1(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    notification: &Value,
) -> Result<u64, Error> {
    let id = entry_string(notification, "id");
    let created_at_snake = entry_string(notification, "created_at");
    let created_at_camel = entry_string(notification, "createdAt");
    let created_at =
        first_non_empty([created_at_snake.as_str(), created_at_camel.as_str()]).to_string();
    let notification_type = entry_string(notification, "type");
    if id.is_empty() || created_at.is_empty() || notification_type.is_empty() {
        return Err(Error::InvalidData(
            "Notification v1 upsert requires id, createdAt/created_at, and type.".into(),
        ));
    }
    let details = notification.get("details").unwrap_or(&Value::Null);
    tx.execute_non_query(
        &format!("INSERT OR IGNORE INTO {user_prefix}_notifications (id, created_at, type, sender_user_id, sender_username, receiver_user_id, message, world_id, world_name, image_url, invite_message, request_message, response_message, expired) VALUES (@id, @created_at, @type, @sender_user_id, @sender_username, @receiver_user_id, @message, @world_id, @world_name, @image_url, @invite_message, @request_message, @response_message, @expired)"),
        &ParamsBuilder::new()
            .set("id", id)
            .set("created_at", created_at)
            .set("type", notification_type)
            .set("sender_user_id", entry_string(notification, "senderUserId"))
            .set("sender_username", entry_string(notification, "senderUsername"))
            .set("receiver_user_id", entry_string(notification, "receiverUserId"))
            .set("message", entry_string(notification, "message"))
            .set("world_id", entry_string(details, "worldId"))
            .set("world_name", entry_string(details, "worldName"))
            .set("image_url", {
                let details_image = entry_string(details, "imageUrl");
                let notification_image = entry_string(notification, "imageUrl");
                first_non_empty([details_image.as_str(), notification_image.as_str()])
                    .to_string()
            })
            .set("invite_message", entry_string(details, "inviteMessage"))
            .set("request_message", entry_string(details, "requestMessage"))
            .set("response_message", entry_string(details, "responseMessage"))
            .set(
                "expired",
                if bool_field(notification.get("$isExpired"))
                    || bool_field(notification.get("expired"))
                {
                    1
                } else {
                    0
                },
            )
            .build(),
    )
    .map(affected_count)
}

fn upsert_notification_v2(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    notification: &Value,
) -> Result<u64, Error> {
    let id = entry_string(notification, "id");
    let created_at = entry_string(notification, "createdAt");
    let notification_type = entry_string(notification, "type");
    if id.is_empty() || created_at.is_empty() || notification_type.is_empty() {
        return Err(Error::InvalidData(
            "Notification v2 upsert requires id, createdAt, and type.".into(),
        ));
    }
    tx.execute_non_query(
        &format!("INSERT OR REPLACE INTO {user_prefix}_notifications_v2 (id, created_at, updated_at, expires_at, type, link, link_text, message, title, image_url, seen, sender_user_id, sender_username, data, responses, details) VALUES (@id, @created_at, @updated_at, @expires_at, @type, @link, @link_text, @message, @title, @image_url, @seen, @sender_user_id, @sender_username, @data, @responses, @details)"),
        &ParamsBuilder::new()
            .set("id", id)
            .set("created_at", created_at)
            .set("updated_at", entry_string(notification, "updatedAt"))
            .set("expires_at", entry_string(notification, "expiresAt"))
            .set("type", notification_type)
            .set("link", entry_string(notification, "link"))
            .set("link_text", entry_string(notification, "linkText"))
            .set("message", entry_string(notification, "message"))
            .set("title", entry_string(notification, "title"))
            .set("image_url", entry_string(notification, "imageUrl"))
            .set("seen", if bool_field(notification.get("seen")) { 1 } else { 0 })
            .set("sender_user_id", entry_string(notification, "senderUserId"))
            .set("sender_username", entry_string(notification, "senderUsername"))
            .set("data", json_string(notification.get("data"), "{}"))
            .set("responses", json_string(notification.get("responses"), "[]"))
            .set("details", json_string(notification.get("details"), "{}"))
            .build(),
    )
    .map(affected_count)
}

fn expire_notification(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    entry: &NotificationExpiration,
) -> Result<u64, Error> {
    let id = normalize_user_id(&entry.id);
    if id.is_empty() {
        return Ok(0);
    }
    let mut affected = affected_count(tx.execute_non_query(
        &format!("UPDATE {user_prefix}_notifications_v2 SET expires_at = @expires_at, seen = 1 WHERE id = @id"),
        &ParamsBuilder::new()
            .set("id", id.clone())
            .set("expires_at", entry.expired_at.clone())
            .build(),
    )?);
    affected = affected.saturating_add(affected_count(tx.execute_non_query(
        &format!("UPDATE {user_prefix}_notifications SET expired = 1 WHERE id = @id"),
        &ParamsBuilder::new().set("id", id).build(),
    )?));
    Ok(affected)
}

fn update_notification_v2(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    entry: &NotificationV2Update,
) -> Result<u64, Error> {
    let id = normalize_user_id(&entry.id);
    let Some(updates) = entry.updates.as_object() else {
        return Ok(0);
    };
    if id.is_empty() || updates.is_empty() {
        return Ok(0);
    }

    let mut assignments = Vec::new();
    let mut params = ParamsBuilder::new().set("id", id.clone());
    for (json_key, column) in [
        ("createdAt", "created_at"),
        ("updatedAt", "updated_at"),
        ("expiresAt", "expires_at"),
        ("type", "type"),
        ("link", "link"),
        ("linkText", "link_text"),
        ("message", "message"),
        ("title", "title"),
        ("imageUrl", "image_url"),
        ("senderUserId", "sender_user_id"),
        ("senderUsername", "sender_username"),
    ] {
        if let Some(value) = updates.get(json_key) {
            assignments.push(format!("{column} = @{column}"));
            params = params.set(column, value.clone());
        }
    }
    if let Some(value) = updates.get("seen") {
        assignments.push("seen = @seen".to_string());
        params = params.set("seen", if bool_field(Some(value)) { 1 } else { 0 });
    }
    for (json_key, column, default) in [
        ("data", "data", "{}"),
        ("responses", "responses", "[]"),
        ("details", "details", "{}"),
    ] {
        if updates.contains_key(json_key) {
            assignments.push(format!("{column} = @{column}"));
            params = params.set(column, json_string(updates.get(json_key), default));
        }
    }

    if assignments.is_empty() {
        return Ok(0);
    }
    let updated = tx.execute_non_query(
        &format!(
            "UPDATE {user_prefix}_notifications_v2 SET {} WHERE id = @id",
            assignments.join(", ")
        ),
        &params.build(),
    )?;
    if updated <= 0 {
        let mut notification = updates.clone();
        notification.insert("id".into(), Value::String(id));
        notification
            .entry("createdAt")
            .or_insert_with(|| Value::String(entry.received_at.clone()));
        notification
            .entry("created_at")
            .or_insert_with(|| Value::String(entry.received_at.clone()));
        return upsert_notification_v2(tx, user_prefix, &Value::Object(notification));
    }
    Ok(affected_count(updated))
}

fn mark_notification_seen(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    id: &str,
) -> Result<u64, Error> {
    let id = normalize_user_id(id);
    if id.is_empty() {
        return Ok(0);
    }
    tx.execute_non_query(
        &format!("UPDATE {user_prefix}_notifications_v2 SET seen = 1 WHERE id = @id"),
        &ParamsBuilder::new().set("id", id).build(),
    )
    .map(affected_count)
}

fn upsert_avatar_history(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    entry: &AvatarHistoryUpsert,
) -> Result<u64, Error> {
    let avatar_id = normalize_user_id(&entry.avatar_id);
    if avatar_id.is_empty() {
        return Ok(0);
    }
    tx.execute_non_query(
        &format!(
            "INSERT INTO {user_prefix}_avatar_history (avatar_id, created_at, time)
             VALUES (@avatar_id, @created_at, 0)
             ON CONFLICT(avatar_id) DO UPDATE SET created_at = @created_at"
        ),
        &ParamsBuilder::new()
            .set("avatar_id", avatar_id)
            .set("created_at", entry.created_at.clone())
            .build(),
    )
    .map(affected_count)
}

fn upsert_avatar_time_spent(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
    entry: &AvatarTimeSpentUpsert,
) -> Result<u64, Error> {
    let avatar_id = normalize_user_id(&entry.avatar_id);
    if avatar_id.is_empty() || entry.time_spent <= 0 {
        return Ok(0);
    }
    tx.execute_non_query(
        &format!(
            "INSERT INTO {user_prefix}_avatar_history (avatar_id, created_at, time)
             VALUES (@avatar_id, @created_at, @time_spent)
             ON CONFLICT(avatar_id) DO UPDATE SET time = time + @time_spent"
        ),
        &ParamsBuilder::new()
            .set("avatar_id", avatar_id)
            .set("created_at", entry.created_at.clone())
            .set("time_spent", entry.time_spent)
            .build(),
    )
    .map(affected_count)
}

fn insert_game_log_location_if_changed(
    tx: &mut DatabaseWriteTransaction<'_>,
    entry: &GameLogLocationEntry,
) -> Result<u64, Error> {
    if entry.location.trim().is_empty() {
        return Ok(0);
    }
    let rows = tx.execute(
        "SELECT location FROM gamelog_location ORDER BY created_at DESC, id DESC LIMIT 1",
        &Default::default(),
    )?;
    let latest = rows
        .first()
        .and_then(|row| row.first())
        .and_then(Value::as_str)
        .unwrap_or("");
    if latest == entry.location {
        return Ok(0);
    }
    tx.execute_non_query(
        "INSERT OR IGNORE INTO gamelog_location (created_at, location, world_id, world_name, time, group_name) VALUES (@created_at, @location, @world_id, @world_name, @time, @group_name)",
        &ParamsBuilder::new()
            .set("created_at", entry.created_at.clone())
            .set("location", entry.location.clone())
            .set("world_id", entry.world_id.clone())
            .set("world_name", entry.world_name.clone())
            .set("time", entry.time)
            .set("group_name", entry.group_name.clone())
            .build(),
    )
    .map(affected_count)
}

fn affected_count(count: i64) -> u64 {
    count.max(0) as u64
}

fn next_friend_number(
    tx: &mut DatabaseWriteTransaction<'_>,
    user_prefix: &str,
) -> Result<i64, Error> {
    let rows = tx.execute(
        &format!("SELECT MAX(friend_number), COUNT(*) FROM {user_prefix}_friend_log_current"),
        &Default::default(),
    )?;
    let max_number = rows
        .first()
        .and_then(|row| row.first())
        .and_then(value_to_i64)
        .unwrap_or(0);
    let count = rows
        .first()
        .and_then(|row| row.get(1))
        .and_then(value_to_i64)
        .unwrap_or(0);
    Ok(if max_number > 0 {
        max_number + 1
    } else {
        count + 1
    })
}

fn existing_friend_log_row(row: &[Value]) -> ExistingFriendLogRow {
    ExistingFriendLogRow {
        user_id: row
            .first()
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        display_name: row.get(1).and_then(Value::as_str).unwrap_or("").to_string(),
        trust_level: row
            .get(2)
            .and_then(Value::as_str)
            .unwrap_or("Visitor")
            .to_string(),
        friend_number: row.get(3).and_then(value_to_i64).unwrap_or(0),
    }
}

fn normalize_user_id(value: &str) -> String {
    value.trim().to_string()
}

fn entry_string(entry: &Value, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            entry
                .get(key)
                .filter(|value| !value.is_null())
                .map(ToString::to_string)
                .unwrap_or_default()
        })
}

fn entry_i64(entry: &Value, key: &str) -> i64 {
    entry.get(key).and_then(value_to_i64).unwrap_or(0)
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| {
            value
                .as_str()
                .and_then(|value| value.trim().parse::<i64>().ok())
        })
}

fn bool_field(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(false)
}

fn json_string(value: Option<&Value>, default: &str) -> String {
    value
        .filter(|value| !value.is_null())
        .map(ToString::to_string)
        .unwrap_or_else(|| default.to_string())
}

fn first_non_empty<'a>(values: impl IntoIterator<Item = &'a str>) -> &'a str {
    values
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("")
        .trim()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use crate::common::ParamsBuilder;
    use crate::database::DatabaseService;
    use crate::game_log::GameLogLocationEntry;

    use super::{
        normalize_user_table_prefix, write_realtime_batch, FriendLogUpsert, NotificationV2Update,
        RealtimePersistenceBatch,
    };

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("vrcx-0-{name}-{}-{nonce}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn normalizes_user_table_prefix_like_frontend() {
        assert_eq!(
            normalize_user_table_prefix("usr_123-abc").unwrap(),
            "usr123abc"
        );
        assert_eq!(normalize_user_table_prefix("123").unwrap(), "_123");
    }

    #[test]
    fn rejects_empty_or_injection_user_table_prefix() {
        for user_id in [
            "",
            "   ",
            "usr_self;DROP TABLE usrself_feed_gps",
            "usr_self feed_gps",
            "usr_self.feed_gps",
            "usr_self/feed_gps",
        ] {
            assert!(
                normalize_user_table_prefix(user_id).is_err(),
                "{user_id:?} must not become a table prefix"
            );
        }
    }

    #[test]
    fn writes_friend_log_and_feed_rows() -> Result<(), crate::Error> {
        let dir = TestDir::new("realtime-persistence");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;
        let counts = write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                friend_log_upserts: vec![FriendLogUpsert {
                    target_user_id: "usr_friend".into(),
                    display_name: "Friend".into(),
                    trust_level: "Known".into(),
                    friend_number: 12,
                    created_at: "2026-05-15T00:00:00Z".into(),
                    force_history: false,
                }],
                feed_entries: vec![json!({
                    "created_at": "2026-05-15T00:00:00Z",
                    "type": "Online",
                    "userId": "usr_friend",
                    "displayName": "Friend",
                    "location": "wrld_1:123",
                    "worldName": "wrld_1",
                    "time": 0,
                    "groupName": ""
                })],
                ..RealtimePersistenceBatch::default()
            },
        )?;
        assert_eq!(counts.affected_count, 3);
        assert_eq!(counts.game_log_affected_count, 0);

        let current = db.execute(
            "SELECT user_id, display_name, trust_level, friend_number FROM usrself_friend_log_current WHERE user_id = @user_id",
            &ParamsBuilder::new().set("user_id", "usr_friend").build(),
        )?;
        assert_eq!(current[0][1], json!("Friend"));
        assert_eq!(current[0][3], json!(12));
        let feed = db.execute(
            "SELECT user_id, type, location FROM usrself_feed_online_offline WHERE user_id = @user_id",
            &ParamsBuilder::new().set("user_id", "usr_friend").build(),
        )?;
        assert_eq!(feed[0][1], json!("Online"));
        assert_eq!(feed[0][2], json!("wrld_1:123"));

        let location_counts = write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                game_log_locations: vec![GameLogLocationEntry {
                    created_at: "2026-05-15T00:00:05Z".into(),
                    location: "wrld_1:123".into(),
                    world_id: "wrld_1".into(),
                    world_name: "World".into(),
                    time: 0,
                    group_name: "".into(),
                }],
                ..RealtimePersistenceBatch::default()
            },
        )?;
        assert_eq!(location_counts.affected_count, 1);
        assert_eq!(location_counts.game_log_affected_count, 1);
        Ok(())
    }

    #[test]
    fn force_history_false_skips_history_on_update() -> Result<(), crate::Error> {
        let dir = TestDir::new("realtime-force-history-false");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;

        let upsert = |name: &str| RealtimePersistenceBatch {
            friend_log_upserts: vec![FriendLogUpsert {
                target_user_id: "usr_friend".into(),
                display_name: name.into(),
                trust_level: "Known".into(),
                friend_number: 12,
                created_at: "2026-05-15T00:00:00Z".into(),
                force_history: false,
            }],
            ..RealtimePersistenceBatch::default()
        };

        write_realtime_batch(&db, "usr_self", &upsert("Friend"))?;
        write_realtime_batch(&db, "usr_self", &upsert("Friend Renamed"))?;

        let history = db.execute(
            "SELECT user_id FROM usrself_friend_log_history WHERE user_id = @user_id",
            &ParamsBuilder::new().set("user_id", "usr_friend").build(),
        )?;
        assert_eq!(history.len(), 1);
        Ok(())
    }

    #[test]
    fn blank_display_name_persists_unknown_not_user_id() -> Result<(), crate::Error> {
        let dir = TestDir::new("realtime-unknown-display-name");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;

        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                friend_log_upserts: vec![FriendLogUpsert {
                    target_user_id: "usr_friend".into(),
                    display_name: String::new(),
                    trust_level: "Known".into(),
                    friend_number: 12,
                    created_at: "2026-05-15T00:00:00Z".into(),
                    force_history: false,
                }],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let current = db.execute(
            "SELECT display_name FROM usrself_friend_log_current WHERE user_id = @user_id",
            &ParamsBuilder::new().set("user_id", "usr_friend").build(),
        )?;
        assert_eq!(current[0][0], json!("Unknown"));
        Ok(())
    }

    #[test]
    fn rejects_invalid_realtime_feed_entry_type() {
        let dir = TestDir::new("realtime-invalid-feed");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap();

        let error = write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                feed_entries: vec![json!({
                    "created_at": "2026-05-15T00:00:00Z",
                    "type": "UnknownFeedType",
                })],
                ..RealtimePersistenceBatch::default()
            },
        )
        .unwrap_err();

        assert!(matches!(error, crate::Error::InvalidData(_)));
    }

    #[test]
    fn rolls_back_friend_log_rows_when_later_feed_entry_fails() -> Result<(), crate::Error> {
        let dir = TestDir::new("realtime-rollback-feed");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;

        let error = write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                friend_log_upserts: vec![FriendLogUpsert {
                    target_user_id: "usr_friend".into(),
                    display_name: "Friend".into(),
                    trust_level: "Known".into(),
                    friend_number: 1,
                    created_at: "2026-05-15T00:00:00Z".into(),
                    force_history: false,
                }],
                feed_entries: vec![json!({
                    "created_at": "2026-05-15T00:00:01Z",
                    "type": "NewFeedType",
                    "userId": "usr_friend",
                })],
                ..RealtimePersistenceBatch::default()
            },
        )
        .unwrap_err();
        assert!(matches!(error, crate::Error::InvalidData(_)));

        let current = db.execute(
            "SELECT COUNT(*) FROM usrself_friend_log_current",
            &Default::default(),
        )?;
        let history = db.execute(
            "SELECT COUNT(*) FROM usrself_friend_log_history",
            &Default::default(),
        )?;
        assert_eq!(current[0][0], json!(0));
        assert_eq!(history[0][0], json!(0));
        Ok(())
    }

    #[test]
    fn writes_notification_v1_and_v2_schema_columns() -> Result<(), crate::Error> {
        let dir = TestDir::new("realtime-notification-columns");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;

        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                notification_v1_upserts: vec![json!({
                    "id": "notif_v1",
                    "createdAt": "2026-05-15T00:00:00Z",
                    "type": "invite",
                    "senderUserId": "usr_sender",
                    "senderUsername": "Sender",
                    "receiverUserId": "usr_self",
                    "message": "Join me",
                    "imageUrl": "https://images.example/fallback.png",
                    "$isExpired": true,
                    "details": {
                        "worldId": "wrld_invite",
                        "worldName": "Invite World",
                        "imageUrl": "https://images.example/details.png",
                        "inviteMessage": "Invite text",
                        "requestMessage": "Request text",
                        "responseMessage": "Response text"
                    }
                })],
                notification_v2_upserts: vec![json!({
                    "id": "notif_v2",
                    "createdAt": "2026-05-15T00:01:00Z",
                    "updatedAt": "2026-05-15T00:01:01Z",
                    "expiresAt": "2026-05-16T00:01:00Z",
                    "type": "friendRequest",
                    "link": "https://vrchat.com/home/user/usr_sender",
                    "linkText": "Open user",
                    "message": "Add me",
                    "title": "Friend request",
                    "imageUrl": "https://images.example/v2.png",
                    "seen": true,
                    "senderUserId": "usr_sender_v2",
                    "senderUsername": "Sender Two",
                    "data": { "groupName": "Group Alpha" },
                    "responses": [{ "type": "accept" }],
                    "details": { "worldId": "wrld_v2", "worldName": "V2 World" }
                })],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let v1 = db.execute(
            concat!(
                "SELECT created_at, type, sender_user_id, sender_username, receiver_user_id, ",
                "message, world_id, world_name, image_url, invite_message, request_message, ",
                "response_message, expired FROM usrself_notifications WHERE id = @id"
            ),
            &ParamsBuilder::new().set("id", "notif_v1").build(),
        )?;
        assert_eq!(v1[0][0], json!("2026-05-15T00:00:00Z"));
        assert_eq!(v1[0][1], json!("invite"));
        assert_eq!(v1[0][2], json!("usr_sender"));
        assert_eq!(v1[0][3], json!("Sender"));
        assert_eq!(v1[0][4], json!("usr_self"));
        assert_eq!(v1[0][5], json!("Join me"));
        assert_eq!(v1[0][6], json!("wrld_invite"));
        assert_eq!(v1[0][7], json!("Invite World"));
        assert_eq!(v1[0][8], json!("https://images.example/details.png"));
        assert_eq!(v1[0][9], json!("Invite text"));
        assert_eq!(v1[0][10], json!("Request text"));
        assert_eq!(v1[0][11], json!("Response text"));
        assert_eq!(v1[0][12], json!(1));

        let v2 = db.execute(
            concat!(
                "SELECT created_at, updated_at, expires_at, type, link, link_text, message, ",
                "title, image_url, seen, sender_user_id, sender_username, data, responses, ",
                "details FROM usrself_notifications_v2 WHERE id = @id"
            ),
            &ParamsBuilder::new().set("id", "notif_v2").build(),
        )?;
        assert_eq!(v2[0][0], json!("2026-05-15T00:01:00Z"));
        assert_eq!(v2[0][1], json!("2026-05-15T00:01:01Z"));
        assert_eq!(v2[0][2], json!("2026-05-16T00:01:00Z"));
        assert_eq!(v2[0][3], json!("friendRequest"));
        assert_eq!(v2[0][4], json!("https://vrchat.com/home/user/usr_sender"));
        assert_eq!(v2[0][5], json!("Open user"));
        assert_eq!(v2[0][6], json!("Add me"));
        assert_eq!(v2[0][7], json!("Friend request"));
        assert_eq!(v2[0][8], json!("https://images.example/v2.png"));
        assert_eq!(v2[0][9], json!(1));
        assert_eq!(v2[0][10], json!("usr_sender_v2"));
        assert_eq!(v2[0][11], json!("Sender Two"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(v2[0][12].as_str().unwrap())?,
            json!({ "groupName": "Group Alpha" })
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(v2[0][13].as_str().unwrap())?,
            json!([{ "type": "accept" }])
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(v2[0][14].as_str().unwrap())?,
            json!({ "worldId": "wrld_v2", "worldName": "V2 World" })
        );
        Ok(())
    }

    #[test]
    fn notification_v2_update_falls_back_to_upsert_with_received_timestamp(
    ) -> Result<(), crate::Error> {
        let dir = TestDir::new("realtime-notification-update-fallback");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;

        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                notification_v2_updates: vec![NotificationV2Update {
                    id: "notif_update".into(),
                    received_at: "2026-05-15T00:02:00Z".into(),
                    updates: json!({
                        "type": "invite",
                        "message": "Fallback insert",
                        "seen": true,
                        "data": { "groupName": "Inserted" },
                        "responses": [],
                        "details": { "worldId": "wrld_inserted" }
                    }),
                }],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let inserted = db.execute(
            concat!(
                "SELECT created_at, type, message, seen, data, details ",
                "FROM usrself_notifications_v2 WHERE id = @id"
            ),
            &ParamsBuilder::new().set("id", "notif_update").build(),
        )?;
        assert_eq!(inserted[0][0], json!("2026-05-15T00:02:00Z"));
        assert_eq!(inserted[0][1], json!("invite"));
        assert_eq!(inserted[0][2], json!("Fallback insert"));
        assert_eq!(inserted[0][3], json!(1));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(inserted[0][4].as_str().unwrap())?,
            json!({ "groupName": "Inserted" })
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(inserted[0][5].as_str().unwrap())?,
            json!({ "worldId": "wrld_inserted" })
        );
        Ok(())
    }

    #[test]
    fn rejects_notifications_missing_required_fields() {
        let dir = TestDir::new("realtime-invalid-notification");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap();

        let v1_error = write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                notification_v1_upserts: vec![json!({
                    "id": "not_1",
                    "createdAt": "2026-05-15T00:00:00Z",
                })],
                ..RealtimePersistenceBatch::default()
            },
        )
        .unwrap_err();
        assert!(matches!(v1_error, crate::Error::InvalidData(_)));

        let v2_error = write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                notification_v2_upserts: vec![json!({
                    "id": "not_2",
                    "type": "invite",
                })],
                ..RealtimePersistenceBatch::default()
            },
        )
        .unwrap_err();
        assert!(matches!(v2_error, crate::Error::InvalidData(_)));
    }
}
