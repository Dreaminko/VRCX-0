use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::common::{add_list_params, normalize_text, value_as_string};
use crate::database::DatabaseService;
use crate::realtime::{ensure_realtime_tables, normalize_user_table_prefix};
use crate::Error;

use super::types::*;

fn query_feed_rows(
    db: &DatabaseService,
    query: &FeedRowsQueryInput,
    should_interrupt: Option<Box<dyn Fn() -> bool + Send + Sync>>,
) -> Result<Vec<FeedRowOutput>, Error> {
    let user_id = normalize_text(&query.user_id);
    let user_prefix = normalize_user_table_prefix(&user_id)?;
    ensure_realtime_tables(db, &user_prefix)?;

    let mut params = HashMap::new();
    let max_entries = if query.max_entries > 0 {
        query.max_entries
    } else {
        500
    };
    params.insert("@limit".into(), Value::from(max_entries));
    params.insert("@per_table".into(), Value::from(max_entries));
    let has_cursor = query
        .cursor
        .as_ref()
        .filter(|cursor| !cursor.created_at.trim().is_empty() && cursor.row_id > 0)
        .is_some();
    if let Some(cursor) = query
        .cursor
        .as_ref()
        .filter(|cursor| !cursor.created_at.trim().is_empty() && cursor.row_id > 0)
    {
        params.insert(
            "@cursor_created_at".into(),
            Value::String(cursor.created_at.clone()),
        );
        params.insert(
            "@cursor_source_rank".into(),
            Value::from(cursor.source_rank),
        );
        params.insert("@cursor_row_id".into(), Value::from(cursor.row_id));
    }

    let vip_placeholders = add_list_params(&mut params, &query.vip_list, "vip");
    let vip_query = if vip_placeholders.is_empty() {
        String::new()
    } else {
        format!("AND user_id IN ({})", vip_placeholders.join(", "))
    };
    let scoped_placeholders = add_list_params(&mut params, &query.scoped_user_ids, "scoped");
    let scoped_query = if scoped_placeholders.is_empty() {
        String::new()
    } else {
        format!("AND user_id IN ({})", scoped_placeholders.join(", "))
    };
    let excluded_placeholders = add_list_params(&mut params, &query.excluded_user_ids, "excluded");
    let excluded_query = if excluded_placeholders.is_empty() {
        String::new()
    } else {
        format!("AND user_id NOT IN ({})", excluded_placeholders.join(", "))
    };
    let user_scope_query = format!("{vip_query} {scoped_query} {excluded_query}");

    let normalize_created_at = should_interrupt.is_some();
    let created_at_expression = if normalize_created_at {
        "julianday(created_at)"
    } else {
        "created_at"
    };
    let cursor_created_at_expression = if normalize_created_at {
        "julianday(@cursor_created_at)"
    } else {
        "@cursor_created_at"
    };
    let search = normalize_text(&query.search);
    let mut date_query = String::new();
    if !query.date_from.trim().is_empty() {
        date_query.push_str(&format!(
            "AND {created_at_expression} >= {date_from_expression} ",
            date_from_expression = if normalize_created_at {
                "julianday(@date_from)"
            } else {
                "@date_from"
            }
        ));
        params.insert("@date_from".into(), Value::String(query.date_from.clone()));
    }
    if !query.date_to.trim().is_empty() {
        date_query.push_str(&format!(
            "AND {created_at_expression} <= {date_to_expression} ",
            date_to_expression = if normalize_created_at {
                "julianday(@date_to)"
            } else {
                "@date_to"
            }
        ));
        params.insert("@date_to".into(), Value::String(query.date_to.clone()));
    }
    let instance_mode = query.mode == FeedQueryMode::Instance
        || (query.mode == FeedQueryMode::Search
            && (search.starts_with("wrld_") || search.starts_with("grp_")));
    let recent_order_sql = format!("{created_at_expression} DESC, id DESC");
    let flags = feed_filter_flags(&query.filters, !instance_mode);
    let mut selects = Vec::new();

    if instance_mode {
        params.insert(
            "@instance_like".into(),
            Value::String(literal_like_pattern(&search)),
        );
        if flags.gps {
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_gps",
                FEED_GPS_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_GPS_SOURCE_RANK,
                    where_sql: &format!(
                        "(location LIKE @instance_like ESCAPE '\\' OR previous_location LIKE @instance_like ESCAPE '\\') {date_query} {user_scope_query}"
                    ),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.online || flags.offline {
            let type_filter = match (flags.online, flags.offline) {
                (true, false) => "AND type = 'Online'",
                (false, true) => "AND type = 'Offline'",
                _ => "",
            };
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_online_offline",
                FEED_ONLINE_OFFLINE_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_ONLINE_OFFLINE_SOURCE_RANK,
                    where_sql: &format!(
                        "location LIKE @instance_like ESCAPE '\\' {type_filter} {date_query} {user_scope_query}"
                    ),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
    } else if query.mode == FeedQueryMode::Lookup {
        if flags.gps {
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_gps",
                FEED_GPS_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_GPS_SOURCE_RANK,
                    where_sql: &format!("1=1 {date_query} {user_scope_query}"),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.status {
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_status",
                FEED_STATUS_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_STATUS_SOURCE_RANK,
                    where_sql: &format!("1=1 {date_query} {user_scope_query}"),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.bio {
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_bio",
                FEED_BIO_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_BIO_SOURCE_RANK,
                    where_sql: &format!("1=1 {date_query} {user_scope_query}"),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.avatar {
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_avatar",
                FEED_AVATAR_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_AVATAR_SOURCE_RANK,
                    where_sql: &format!("1=1 {date_query} {user_scope_query}"),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.online || flags.offline {
            let type_filter = match (flags.online, flags.offline) {
                (true, false) => "AND type = 'Online'",
                (false, true) => "AND type = 'Offline'",
                _ => "",
            };
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_online_offline",
                FEED_ONLINE_OFFLINE_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_ONLINE_OFFLINE_SOURCE_RANK,
                    where_sql: &format!("1=1 {type_filter} {date_query} {user_scope_query}"),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
    } else {
        params.insert(
            "@search_like".into(),
            Value::String(literal_like_pattern(&search)),
        );
        if flags.gps {
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_gps",
                FEED_GPS_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_GPS_SOURCE_RANK,
                    where_sql: &format!(
                        "(display_name LIKE @search_like ESCAPE '\\' OR location LIKE @search_like ESCAPE '\\' OR world_name LIKE @search_like ESCAPE '\\' OR previous_location LIKE @search_like ESCAPE '\\' OR group_name LIKE @search_like ESCAPE '\\') {date_query} {user_scope_query}"
                    ),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.status {
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_status",
                FEED_STATUS_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_STATUS_SOURCE_RANK,
                    where_sql: &format!(
                        "(display_name LIKE @search_like ESCAPE '\\' OR status LIKE @search_like ESCAPE '\\' OR status_description LIKE @search_like ESCAPE '\\' OR previous_status LIKE @search_like ESCAPE '\\' OR previous_status_description LIKE @search_like ESCAPE '\\') {date_query} {user_scope_query}"
                    ),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.bio {
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_bio",
                FEED_BIO_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_BIO_SOURCE_RANK,
                    where_sql: &format!(
                        "(display_name LIKE @search_like ESCAPE '\\' OR bio LIKE @search_like ESCAPE '\\' OR previous_bio LIKE @search_like ESCAPE '\\') {date_query} {user_scope_query}"
                    ),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.avatar {
            let avatar_query = match search.as_str() {
                "private" => "OR user_id = owner_id",
                "public" => "OR user_id != owner_id",
                _ => "",
            };
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_avatar",
                FEED_AVATAR_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_AVATAR_SOURCE_RANK,
                    where_sql: &format!(
                        "((display_name LIKE @search_like ESCAPE '\\' OR avatar_name LIKE @search_like ESCAPE '\\') {avatar_query}) {date_query} {user_scope_query}"
                    ),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
        if flags.online || flags.offline {
            let type_filter = match (flags.online, flags.offline) {
                (true, false) => "AND type = 'Online'",
                (false, true) => "AND type = 'Offline'",
                _ => "",
            };
            let where_sql = "(display_name LIKE @search_like ESCAPE '\\' OR location LIKE @search_like ESCAPE '\\' OR world_name LIKE @search_like ESCAPE '\\' OR group_name LIKE @search_like ESCAPE '\\')";
            push_feed_select(
                &mut selects,
                &user_prefix,
                "feed_online_offline",
                FEED_ONLINE_OFFLINE_PROJECTION,
                FeedSelectOptions {
                    source_rank: FEED_ONLINE_OFFLINE_SOURCE_RANK,
                    where_sql: &format!(
                        "{where_sql} {type_filter} {date_query} {user_scope_query}"
                    ),
                    has_cursor,
                    order_sql: &recent_order_sql,
                    created_at_expression,
                    cursor_created_at_expression,
                },
            );
        }
    }

    if selects.is_empty() {
        return Ok(Vec::new());
    }

    let sql = format!(
        "SELECT {} FROM ({}) ORDER BY created_at_sort DESC, source_rank DESC, id DESC LIMIT @limit",
        feed_base_columns(),
        selects.join(" UNION ALL ")
    );
    match should_interrupt {
        Some(callback) => db.execute_interruptible(&sql, &params, callback),
        None => db.execute(&sql, &params),
    }
    .map(|rows| {
        rows.iter()
            .map(|row| feed_row_from_unified_row(row))
            .collect()
    })
}

pub fn feed_rows_query(
    db: &DatabaseService,
    query: FeedRowsQueryInput,
) -> Result<Vec<FeedRowOutput>, Error> {
    query_feed_rows(db, &query, None)
}

pub fn feed_rows_query_interruptible<F>(
    db: &DatabaseService,
    query: FeedRowsQueryInput,
    should_interrupt: F,
) -> Result<Vec<FeedRowOutput>, Error>
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    query_feed_rows(db, &query, Some(Box::new(should_interrupt)))
}

pub fn feed_latest_query(
    db: &DatabaseService,
    query: FeedLatestQueryInput,
    live_entries: Vec<FeedLiveEntryInput>,
    watermark: i64,
    include_persisted_rows: bool,
) -> Result<FeedReadModelOutput, Error> {
    if query.favorites_only && query.favorite_user_ids.is_empty() {
        return Ok(FeedReadModelOutput {
            rows: Vec::new(),
            max_sequence: watermark,
            persisted_cursor: None,
            persisted_has_more: false,
        });
    }
    let rows = if include_persisted_rows {
        query_feed_rows(
            db,
            &FeedRowsQueryInput {
                user_id: query.user_id.clone(),
                mode: FeedQueryMode::Lookup,
                search: String::new(),
                filters: query.filters.clone(),
                vip_list: if query.favorites_only {
                    query.favorite_user_ids.clone()
                } else {
                    Vec::new()
                },
                scoped_user_ids: query.scoped_user_ids.clone(),
                excluded_user_ids: query.excluded_user_ids.clone(),
                max_entries: query.max_rows,
                date_from: String::new(),
                date_to: String::new(),
                cursor: None,
            },
            None,
        )?
    } else {
        Vec::new()
    };
    let persisted_cursor = rows.last().and_then(feed_cursor_from_row);
    let persisted_has_more =
        include_persisted_rows && query.max_rows > 0 && rows.len() >= query.max_rows as usize;
    let context = FeedLiveRowsMergeContext {
        current_user_id: &query.user_id,
        filters: &query.filters,
        search: "",
        date_from: "",
        date_to: "",
        favorites_only: query.favorites_only,
        favorite_user_ids: &query.favorite_user_ids,
        scoped_user_ids: &query.scoped_user_ids,
        excluded_user_ids: &query.excluded_user_ids,
        max_rows: query.max_rows,
    };
    let mut output = merge_feed_rows_with_live(rows, &live_entries, 0, context);
    output.max_sequence = watermark.max(output.max_sequence);
    output.persisted_cursor = persisted_cursor;
    output.persisted_has_more = persisted_has_more;
    Ok(output)
}

pub fn feed_search_query(
    db: &DatabaseService,
    query: FeedSearchQueryInput,
    live_entries: Vec<FeedLiveEntryInput>,
    watermark: i64,
    include_persisted_rows: bool,
) -> Result<FeedReadModelOutput, Error> {
    if query.favorites_only && query.favorite_user_ids.is_empty() {
        return Ok(FeedReadModelOutput {
            rows: Vec::new(),
            max_sequence: watermark,
            persisted_cursor: None,
            persisted_has_more: false,
        });
    }
    let rows = if include_persisted_rows {
        query_feed_rows(
            db,
            &FeedRowsQueryInput {
                user_id: query.user_id.clone(),
                mode: FeedQueryMode::Search,
                search: query.search.clone(),
                filters: query.filters.clone(),
                vip_list: if query.favorites_only {
                    query.favorite_user_ids.clone()
                } else {
                    Vec::new()
                },
                scoped_user_ids: query.scoped_user_ids.clone(),
                excluded_user_ids: query.excluded_user_ids.clone(),
                max_entries: query.max_rows,
                date_from: query.date_from.clone(),
                date_to: query.date_to.clone(),
                cursor: None,
            },
            None,
        )?
    } else {
        Vec::new()
    };
    let context = FeedLiveRowsMergeContext {
        current_user_id: &query.user_id,
        filters: &query.filters,
        search: &query.search,
        date_from: &query.date_from,
        date_to: &query.date_to,
        favorites_only: query.favorites_only,
        favorite_user_ids: &query.favorite_user_ids,
        scoped_user_ids: &query.scoped_user_ids,
        excluded_user_ids: &query.excluded_user_ids,
        max_rows: query.max_rows,
    };
    let mut output = merge_feed_rows_with_live(rows, &live_entries, 0, context);
    output.max_sequence = watermark.max(output.max_sequence);
    Ok(output)
}

const FEED_GPS_SOURCE_RANK: i64 = 60;
const FEED_ONLINE_OFFLINE_SOURCE_RANK: i64 = 50;
const FEED_STATUS_SOURCE_RANK: i64 = 40;
const FEED_AVATAR_SOURCE_RANK: i64 = 30;
const FEED_BIO_SOURCE_RANK: i64 = 20;

const FEED_GPS_PROJECTION: &str = "id, 60 AS source_rank, created_at, user_id, display_name, 'GPS' AS type, location, world_name, previous_location, time, group_name, NULL AS status, NULL AS status_description, NULL AS previous_status, NULL AS previous_status_description, NULL AS bio, NULL AS previous_bio, NULL AS owner_id, NULL AS avatar_name, NULL AS current_avatar_image_url, NULL AS current_avatar_thumbnail_image_url, NULL AS previous_current_avatar_image_url, NULL AS previous_current_avatar_thumbnail_image_url";
const FEED_STATUS_PROJECTION: &str = "id, 40 AS source_rank, created_at, user_id, display_name, 'Status' AS type, NULL AS location, NULL AS world_name, NULL AS previous_location, NULL AS time, NULL AS group_name, status, status_description, previous_status, previous_status_description, NULL AS bio, NULL AS previous_bio, NULL AS owner_id, NULL AS avatar_name, NULL AS current_avatar_image_url, NULL AS current_avatar_thumbnail_image_url, NULL AS previous_current_avatar_image_url, NULL AS previous_current_avatar_thumbnail_image_url";
const FEED_BIO_PROJECTION: &str = "id, 20 AS source_rank, created_at, user_id, display_name, 'Bio' AS type, NULL AS location, NULL AS world_name, NULL AS previous_location, NULL AS time, NULL AS group_name, NULL AS status, NULL AS status_description, NULL AS previous_status, NULL AS previous_status_description, bio, previous_bio, NULL AS owner_id, NULL AS avatar_name, NULL AS current_avatar_image_url, NULL AS current_avatar_thumbnail_image_url, NULL AS previous_current_avatar_image_url, NULL AS previous_current_avatar_thumbnail_image_url";
const FEED_AVATAR_PROJECTION: &str = "id, 30 AS source_rank, created_at, user_id, display_name, 'Avatar' AS type, NULL AS location, NULL AS world_name, NULL AS previous_location, NULL AS time, NULL AS group_name, NULL AS status, NULL AS status_description, NULL AS previous_status, NULL AS previous_status_description, NULL AS bio, NULL AS previous_bio, owner_id, avatar_name, current_avatar_image_url, current_avatar_thumbnail_image_url, previous_current_avatar_image_url, previous_current_avatar_thumbnail_image_url";
const FEED_ONLINE_OFFLINE_PROJECTION: &str = "id, 50 AS source_rank, created_at, user_id, display_name, type, location, world_name, NULL AS previous_location, time, group_name, NULL AS status, NULL AS status_description, NULL AS previous_status, NULL AS previous_status_description, NULL AS bio, NULL AS previous_bio, NULL AS owner_id, NULL AS avatar_name, NULL AS current_avatar_image_url, NULL AS current_avatar_thumbnail_image_url, NULL AS previous_current_avatar_image_url, NULL AS previous_current_avatar_thumbnail_image_url";

struct FeedSelectOptions<'a> {
    source_rank: i64,
    where_sql: &'a str,
    has_cursor: bool,
    order_sql: &'a str,
    created_at_expression: &'a str,
    cursor_created_at_expression: &'a str,
}

fn push_feed_select(
    selects: &mut Vec<String>,
    user_prefix: &str,
    table_suffix: &str,
    projection: &str,
    options: FeedSelectOptions<'_>,
) {
    let cursor_sql = feed_cursor_condition(
        options.source_rank,
        options.has_cursor,
        options.created_at_expression,
        options.cursor_created_at_expression,
    );
    let where_sql = options.where_sql;
    let order_sql = options.order_sql;
    let created_at_expression = options.created_at_expression;
    selects.push(format!(
        "SELECT * FROM (SELECT {projection}, {created_at_expression} AS created_at_sort FROM {user_prefix}_{table_suffix} WHERE {where_sql} {cursor_sql} ORDER BY {order_sql} LIMIT @per_table)"
    ));
}

fn feed_cursor_condition(
    source_rank: i64,
    has_cursor: bool,
    created_at_expression: &str,
    cursor_created_at_expression: &str,
) -> String {
    if !has_cursor {
        return String::new();
    }
    format!(
        "AND ({created_at_expression} < {cursor_created_at_expression} OR ({created_at_expression} = {cursor_created_at_expression} AND {source_rank} < @cursor_source_rank) OR ({created_at_expression} = {cursor_created_at_expression} AND {source_rank} = @cursor_source_rank AND id < @cursor_row_id))"
    )
}

fn value_opt_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Null => None,
        other => {
            let text = value_as_string(other);
            (!text.is_empty()).then_some(text)
        }
    }
}

fn value_opt_i64(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

fn value_opt_string_list(value: Option<&Value>) -> Option<Vec<String>> {
    Some(
        value?
            .as_array()?
            .iter()
            .filter(|item| !item.is_null())
            .map(value_as_string)
            .collect(),
    )
}

fn row_opt_string(row: &[Value], index: usize) -> Option<String> {
    value_opt_string(row.get(index))
}

fn row_opt_i64(row: &[Value], index: usize) -> Option<i64> {
    value_opt_i64(row.get(index))
}

fn entry_opt_string(entry: &Value, keys: &[&str]) -> Option<String> {
    value_opt_string(feed_entry_value(entry, keys))
}

fn entry_opt_i64(entry: &Value, keys: &[&str]) -> Option<i64> {
    value_opt_i64(feed_entry_value(entry, keys))
}

fn entry_opt_string_list(entry: &Value, keys: &[&str]) -> Option<Vec<String>> {
    value_opt_string_list(feed_entry_value(entry, keys))
}

fn feed_row_from_unified_row(row: &[Value]) -> FeedRowOutput {
    FeedRowOutput {
        row_id: row_opt_i64(row, 0),
        source_rank: row_opt_i64(row, 1),
        created_at: row_opt_string(row, 2),
        user_id: row_opt_string(row, 3),
        display_name: row_opt_string(row, 4),
        r#type: row_opt_string(row, 5),
        location: row_opt_string(row, 6),
        world_name: row_opt_string(row, 7),
        previous_location: row_opt_string(row, 8),
        time: row_opt_i64(row, 9),
        group_name: row_opt_string(row, 10),
        status: row_opt_string(row, 11),
        status_description: row_opt_string(row, 12),
        previous_status: row_opt_string(row, 13),
        previous_status_description: row_opt_string(row, 14),
        bio: row_opt_string(row, 15),
        previous_bio: row_opt_string(row, 16),
        owner_id: row_opt_string(row, 17),
        avatar_name: row_opt_string(row, 18),
        current_avatar_image_url: row_opt_string(row, 19),
        current_avatar_thumbnail_image_url: row_opt_string(row, 20),
        current_avatar_tags: None,
        previous_owner_id: None,
        previous_avatar_name: None,
        previous_current_avatar_image_url: row_opt_string(row, 21),
        previous_current_avatar_thumbnail_image_url: row_opt_string(row, 22),
        previous_current_avatar_tags: None,
        owner_user_id: None,
    }
}

fn feed_row_from_value(entry: &Value) -> FeedRowOutput {
    FeedRowOutput {
        row_id: entry_opt_i64(entry, &["rowId", "row_id"]),
        source_rank: entry_opt_i64(entry, &["sourceRank", "source_rank"]),
        created_at: entry_opt_string(entry, &["created_at", "createdAt"]),
        user_id: entry_opt_string(entry, &["userId", "user_id"]),
        display_name: entry_opt_string(entry, &["displayName", "display_name"]),
        r#type: entry_opt_string(entry, &["type"]),
        location: entry_opt_string(entry, &["location"]),
        world_name: entry_opt_string(entry, &["worldName", "world_name"]),
        previous_location: entry_opt_string(entry, &["previousLocation", "previous_location"]),
        time: entry_opt_i64(entry, &["time"]),
        group_name: entry_opt_string(entry, &["groupName", "group_name"]),
        status: entry_opt_string(entry, &["status"]),
        status_description: entry_opt_string(entry, &["statusDescription", "status_description"]),
        previous_status: entry_opt_string(entry, &["previousStatus", "previous_status"]),
        previous_status_description: entry_opt_string(
            entry,
            &["previousStatusDescription", "previous_status_description"],
        ),
        bio: entry_opt_string(entry, &["bio"]),
        previous_bio: entry_opt_string(entry, &["previousBio", "previous_bio"]),
        owner_id: entry_opt_string(entry, &["ownerId", "owner_id"]),
        avatar_name: entry_opt_string(entry, &["avatarName", "avatar_name"]),
        current_avatar_image_url: entry_opt_string(
            entry,
            &["currentAvatarImageUrl", "current_avatar_image_url"],
        ),
        current_avatar_thumbnail_image_url: entry_opt_string(
            entry,
            &[
                "currentAvatarThumbnailImageUrl",
                "current_avatar_thumbnail_image_url",
            ],
        ),
        current_avatar_tags: entry_opt_string_list(
            entry,
            &["currentAvatarTags", "current_avatar_tags"],
        ),
        previous_owner_id: entry_opt_string(entry, &["previousOwnerId", "previous_owner_id"]),
        previous_avatar_name: entry_opt_string(
            entry,
            &["previousAvatarName", "previous_avatar_name"],
        ),
        previous_current_avatar_image_url: entry_opt_string(
            entry,
            &[
                "previousCurrentAvatarImageUrl",
                "previous_current_avatar_image_url",
            ],
        ),
        previous_current_avatar_thumbnail_image_url: entry_opt_string(
            entry,
            &[
                "previousCurrentAvatarThumbnailImageUrl",
                "previous_current_avatar_thumbnail_image_url",
            ],
        ),
        previous_current_avatar_tags: entry_opt_string_list(
            entry,
            &["previousCurrentAvatarTags", "previous_current_avatar_tags"],
        ),
        owner_user_id: entry_opt_string(entry, &["ownerUserId", "owner_user_id"]),
    }
}

#[derive(Default)]
struct FeedFilterFlags {
    pub(crate) gps: bool,
    pub(crate) status: bool,
    pub(crate) bio: bool,
    pub(crate) avatar: bool,
    pub(crate) online: bool,
    pub(crate) offline: bool,
}

fn feed_filter_flags(filters: &[FeedFilter], include_profile: bool) -> FeedFilterFlags {
    let mut flags = FeedFilterFlags {
        gps: true,
        status: include_profile,
        bio: include_profile,
        avatar: include_profile,
        online: true,
        offline: true,
    };
    if filters.is_empty() {
        return flags;
    }

    flags = FeedFilterFlags::default();
    for filter in filters {
        match filter {
            FeedFilter::Gps => flags.gps = true,
            FeedFilter::Status if include_profile => flags.status = true,
            FeedFilter::Bio if include_profile => flags.bio = true,
            FeedFilter::Avatar if include_profile => flags.avatar = true,
            FeedFilter::Online => flags.online = true,
            FeedFilter::Offline => flags.offline = true,
            FeedFilter::Status | FeedFilter::Bio | FeedFilter::Avatar => {}
        }
    }
    flags
}

fn feed_base_columns() -> &'static str {
    "id, source_rank, created_at, user_id, display_name, type, location, world_name, previous_location, time, group_name, status, status_description, previous_status, previous_status_description, bio, previous_bio, owner_id, avatar_name, current_avatar_image_url, current_avatar_thumbnail_image_url, previous_current_avatar_image_url, previous_current_avatar_thumbnail_image_url"
}

fn literal_like_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    format!("%{escaped}%")
}

fn feed_entry_value<'a>(entry: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let object = entry.as_object()?;
    keys.iter()
        .find_map(|key| object.get(*key).filter(|value| !value.is_null()))
}

fn feed_entry_string(entry: &Value, keys: &[&str]) -> String {
    feed_entry_value(entry, keys)
        .map(value_as_string)
        .unwrap_or_default()
}

fn feed_row_key(row: &FeedRowOutput) -> String {
    let entry_type = row.r#type.as_deref().unwrap_or_default();
    if let Some(row_id) = row.row_id {
        return format!("row:{entry_type}:{row_id}");
    }

    feed_row_content_key(row)
}

fn feed_cursor_from_row(row: &FeedRowOutput) -> Option<FeedCursorInput> {
    Some(FeedCursorInput {
        created_at: row.created_at.clone()?,
        source_rank: row.source_rank?,
        row_id: row.row_id?,
    })
}

fn feed_row_content_key(row: &FeedRowOutput) -> String {
    let entry_type = row.r#type.as_deref().unwrap_or_default();
    format!(
        "{entry_type}:{}:{}:{}",
        row.created_at.as_deref().unwrap_or_default(),
        row.user_id.as_deref().unwrap_or_default(),
        row.location.as_deref().unwrap_or_default()
    )
}

fn feed_search_matches(row: &Value, search: &str) -> bool {
    let query = search.trim().to_uppercase();
    if query.is_empty() {
        return true;
    }

    if feed_entry_string(row, &["type"]) == "Avatar" {
        let user_id = feed_entry_string(row, &["userId", "user_id"]);
        let owner_id = feed_entry_string(row, &["ownerId", "owner_id"]);
        if !user_id.is_empty()
            && !owner_id.is_empty()
            && ((query == "PRIVATE" && user_id == owner_id)
                || (query == "PUBLIC" && user_id != owner_id))
        {
            return true;
        }
    }

    if (query.starts_with("WRLD_") || query.starts_with("GRP_"))
        && feed_entry_string(row, &["location"])
            .to_uppercase()
            .contains(&query)
    {
        return true;
    }

    [
        feed_entry_string(row, &["displayName", "display_name"]),
        feed_entry_string(row, &["worldName", "world_name"]),
        feed_entry_string(row, &["groupName", "group_name"]),
        feed_entry_string(row, &["status"]),
        feed_entry_string(row, &["statusDescription", "status_description"]),
        feed_entry_string(row, &["previousStatus", "previous_status"]),
        feed_entry_string(
            row,
            &["previousStatusDescription", "previous_status_description"],
        ),
        feed_entry_string(row, &["bio"]),
        feed_entry_string(row, &["previousBio", "previous_bio"]),
        feed_entry_string(row, &["avatarName", "avatar_name"]),
        feed_entry_string(row, &["message"]),
    ]
    .iter()
    .any(|value| value.to_uppercase().contains(&query))
}

pub struct FeedLiveQueryMatcher {
    current_user_id: String,
    filters: Vec<FeedFilter>,
    search: String,
    date_from: String,
    date_to: String,
    favorites_only: bool,
    favorite_user_ids: HashSet<String>,
    scoped_user_ids: HashSet<String>,
    excluded_user_ids: HashSet<String>,
    max_rows: Option<usize>,
}

impl FeedLiveQueryMatcher {
    pub fn for_latest(query: &FeedLatestQueryInput) -> Self {
        Self {
            current_user_id: query.user_id.clone(),
            filters: query.filters.clone(),
            search: String::new(),
            date_from: String::new(),
            date_to: String::new(),
            favorites_only: query.favorites_only,
            favorite_user_ids: normalize_user_ids(&query.favorite_user_ids),
            scoped_user_ids: normalize_user_ids(&query.scoped_user_ids),
            excluded_user_ids: normalize_user_ids(&query.excluded_user_ids),
            max_rows: (query.max_rows > 0).then_some(query.max_rows as usize),
        }
    }

    pub fn for_search(query: &FeedSearchQueryInput) -> Self {
        Self {
            current_user_id: query.user_id.clone(),
            filters: query.filters.clone(),
            search: query.search.clone(),
            date_from: query.date_from.clone(),
            date_to: query.date_to.clone(),
            favorites_only: query.favorites_only,
            favorite_user_ids: normalize_user_ids(&query.favorite_user_ids),
            scoped_user_ids: normalize_user_ids(&query.scoped_user_ids),
            excluded_user_ids: normalize_user_ids(&query.excluded_user_ids),
            max_rows: (query.max_rows > 0).then_some(query.max_rows as usize),
        }
    }

    fn from_merge_context(context: &FeedLiveRowsMergeContext<'_>) -> Self {
        Self {
            current_user_id: context.current_user_id.to_string(),
            filters: context.filters.to_vec(),
            search: context.search.to_string(),
            date_from: context.date_from.to_string(),
            date_to: context.date_to.to_string(),
            favorites_only: context.favorites_only,
            favorite_user_ids: normalize_user_ids(context.favorite_user_ids),
            scoped_user_ids: normalize_user_ids(context.scoped_user_ids),
            excluded_user_ids: normalize_user_ids(context.excluded_user_ids),
            max_rows: (context.max_rows > 0).then_some(context.max_rows as usize),
        }
    }

    pub fn matches(&self, row: &Value) -> bool {
        if !row.is_object() {
            return false;
        }

        let entry_type = feed_entry_string(row, &["type"]);
        let Some(entry_filter) = FeedFilter::from_event_type(&entry_type) else {
            return false;
        };

        let owner_user_id = feed_entry_string(row, &["ownerUserId", "owner_user_id"]);
        if !owner_user_id.is_empty() && owner_user_id != self.current_user_id {
            return false;
        }

        if !self.filters.is_empty() && !self.filters.contains(&entry_filter) {
            return false;
        }

        let user_id = feed_entry_string(row, &["userId", "user_id"]);
        if self.favorites_only && (user_id.is_empty() || !self.favorite_user_ids.contains(&user_id))
        {
            return false;
        }
        if !self.scoped_user_ids.is_empty() && !self.scoped_user_ids.contains(&user_id) {
            return false;
        }
        if !user_id.is_empty() && self.excluded_user_ids.contains(&user_id) {
            return false;
        }

        let created_at = feed_entry_string(row, &["created_at", "createdAt"]);
        if !self.date_from.trim().is_empty()
            && !created_at.is_empty()
            && created_at.as_str() < self.date_from.as_str()
        {
            return false;
        }
        if !self.date_to.trim().is_empty()
            && !created_at.is_empty()
            && created_at.as_str() > self.date_to.as_str()
        {
            return false;
        }

        feed_search_matches(row, &self.search)
    }

    pub fn max_rows(&self) -> Option<usize> {
        self.max_rows
    }
}

fn normalize_user_ids(user_ids: &[String]) -> HashSet<String> {
    user_ids
        .iter()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .collect()
}

pub(crate) struct FeedLiveRowsMergeContext<'a> {
    pub(crate) current_user_id: &'a str,
    pub(crate) filters: &'a [FeedFilter],
    pub(crate) search: &'a str,
    pub(crate) date_from: &'a str,
    pub(crate) date_to: &'a str,
    pub(crate) favorites_only: bool,
    pub(crate) favorite_user_ids: &'a [String],
    pub(crate) scoped_user_ids: &'a [String],
    pub(crate) excluded_user_ids: &'a [String],
    pub(crate) max_rows: i64,
}

fn merge_feed_rows_with_live(
    rows: Vec<FeedRowOutput>,
    live_entries: &[FeedLiveEntryInput],
    min_live_sequence: i64,
    context: FeedLiveRowsMergeContext<'_>,
) -> FeedReadModelOutput {
    let matcher = FeedLiveQueryMatcher::from_merge_context(&context);
    let mut max_sequence = min_live_sequence;
    let mut matching_entries = Vec::new();

    for live_entry in live_entries
        .iter()
        .filter(|entry| entry.sequence > min_live_sequence)
    {
        max_sequence = max_sequence.max(live_entry.sequence);
        if matcher.matches(live_entry.entry.as_value()) {
            matching_entries.push(feed_row_from_value(live_entry.entry.as_value()));
        }
    }

    let max_rows = if context.max_rows > 0 {
        context.max_rows as usize
    } else {
        rows.len().saturating_add(matching_entries.len())
    };
    let mut live_content_keys = HashSet::new();
    let mut persisted_row_keys = HashSet::new();
    let mut output_rows = Vec::new();

    for entry in matching_entries.into_iter().rev() {
        if live_content_keys.insert(feed_row_content_key(&entry)) {
            output_rows.push(entry);
        }
    }
    for row in rows {
        if let Some(user_id) = row.user_id.as_ref() {
            if !matcher.scoped_user_ids.is_empty() && !matcher.scoped_user_ids.contains(user_id) {
                continue;
            }
            if !user_id.is_empty() && matcher.excluded_user_ids.contains(user_id) {
                continue;
            }
        }
        if live_content_keys.contains(&feed_row_content_key(&row)) {
            continue;
        }
        if persisted_row_keys.insert(feed_row_key(&row)) {
            output_rows.push(row);
        }
    }
    output_rows.truncate(max_rows);

    FeedReadModelOutput {
        rows: output_rows,
        max_sequence,
        persisted_cursor: None,
        persisted_has_more: false,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;
    use vrcx_0_core::json::RawJson;

    use super::{
        feed_latest_query, feed_row_from_value, feed_rows_query, feed_rows_query_interruptible,
        merge_feed_rows_with_live, FeedCursorInput, FeedFilter, FeedLatestQueryInput,
        FeedLiveEntryInput, FeedLiveRowsMergeContext, FeedQueryMode, FeedReadModelOutput,
        FeedRowsQueryInput,
    };
    use crate::database::DatabaseService;
    use crate::realtime::{write_realtime_batch, RealtimePersistenceBatch};

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

    #[derive(Default)]
    struct MergeCase {
        rows: Vec<RawJson>,
        current_user_id: String,
        filters: Vec<FeedFilter>,
        search: String,
        date_from: String,
        date_to: String,
        favorites_only: bool,
        favorite_user_ids: Vec<String>,
        scoped_user_ids: Vec<String>,
        excluded_user_ids: Vec<String>,
        live_entries: Vec<FeedLiveEntryInput>,
        min_live_sequence: i64,
        max_rows: i64,
    }

    fn merge_case(input: MergeCase) -> FeedReadModelOutput {
        let context = FeedLiveRowsMergeContext {
            current_user_id: &input.current_user_id,
            filters: &input.filters,
            search: &input.search,
            date_from: &input.date_from,
            date_to: &input.date_to,
            favorites_only: input.favorites_only,
            favorite_user_ids: &input.favorite_user_ids,
            scoped_user_ids: &input.scoped_user_ids,
            excluded_user_ids: &input.excluded_user_ids,
            max_rows: input.max_rows,
        };
        merge_feed_rows_with_live(
            input
                .rows
                .iter()
                .map(|row| feed_row_from_value(row.as_value()))
                .collect(),
            &input.live_entries,
            input.min_live_sequence,
            context,
        )
    }

    #[test]
    fn live_feed_ignores_friend_relationship_events_without_active_filters() {
        let output = merge_case(MergeCase {
            rows: Vec::new(),
            current_user_id: "usr_self".into(),
            filters: Vec::new(),
            search: String::new(),
            date_from: String::new(),
            date_to: String::new(),
            favorites_only: false,
            favorite_user_ids: Vec::new(),
            scoped_user_ids: Vec::new(),
            excluded_user_ids: Vec::new(),
            live_entries: vec![
                FeedLiveEntryInput {
                    sequence: 1,
                    entry: RawJson::from(json!({
                        "type": "Friend",
                        "userId": "usr_friend",
                        "displayName": "Friend",
                        "created_at": "2026-05-15T00:00:00Z",
                    })),
                },
                FeedLiveEntryInput {
                    sequence: 2,
                    entry: RawJson::from(json!({
                        "type": "GPS",
                        "userId": "usr_friend",
                        "displayName": "Friend",
                        "location": "wrld_1:instance",
                        "created_at": "2026-05-15T00:00:01Z",
                    })),
                },
            ],
            min_live_sequence: 0,
            max_rows: 10,
        });

        assert_eq!(output.max_sequence, 2);
        assert_eq!(output.rows.len(), 1);
        assert_eq!(output.rows[0].r#type.as_deref(), Some("GPS"));
    }

    #[test]
    fn user_scope_drops_live_entries_and_existing_rows_outside_the_scope() {
        let output = merge_case(MergeCase {
            rows: vec![
                RawJson::from(json!({
                    "type": "GPS",
                    "userId": "usr_scoped",
                    "displayName": "Scoped",
                    "location": "wrld_1:instance",
                    "created_at": "2026-05-15T00:00:00Z",
                })),
                RawJson::from(json!({
                    "type": "GPS",
                    "userId": "usr_other",
                    "displayName": "Other",
                    "location": "wrld_2:instance",
                    "created_at": "2026-05-15T00:00:01Z",
                })),
            ],
            current_user_id: "usr_self".into(),
            filters: Vec::new(),
            search: String::new(),
            date_from: String::new(),
            date_to: String::new(),
            favorites_only: false,
            favorite_user_ids: Vec::new(),
            scoped_user_ids: vec!["usr_scoped".into()],
            excluded_user_ids: Vec::new(),
            live_entries: vec![
                FeedLiveEntryInput {
                    sequence: 1,
                    entry: RawJson::from(json!({
                        "type": "GPS",
                        "userId": "usr_other",
                        "displayName": "Other",
                        "location": "wrld_3:instance",
                        "created_at": "2026-05-15T00:00:02Z",
                    })),
                },
                FeedLiveEntryInput {
                    sequence: 2,
                    entry: RawJson::from(json!({
                        "type": "GPS",
                        "userId": "usr_scoped",
                        "displayName": "Scoped",
                        "location": "wrld_4:instance",
                        "created_at": "2026-05-15T00:00:03Z",
                    })),
                },
            ],
            min_live_sequence: 0,
            max_rows: 10,
        });

        assert_eq!(output.max_sequence, 2);
        let user_ids = output
            .rows
            .iter()
            .map(|row| row.user_id.as_deref().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(user_ids, vec!["usr_scoped", "usr_scoped"]);
    }

    #[test]
    fn live_feed_rows_keep_avatar_fields_that_only_exist_on_live_entries() {
        let output = merge_case(MergeCase {
            rows: Vec::new(),
            current_user_id: "usr_self".into(),
            filters: Vec::new(),
            search: String::new(),
            date_from: String::new(),
            date_to: String::new(),
            favorites_only: false,
            favorite_user_ids: Vec::new(),
            scoped_user_ids: Vec::new(),
            excluded_user_ids: Vec::new(),
            live_entries: vec![FeedLiveEntryInput {
                sequence: 1,
                entry: RawJson::from(json!({
                    "type": "Avatar",
                    "userId": "usr_friend",
                    "displayName": "Friend",
                    "created_at": "2026-05-15T00:00:00Z",
                    "avatarName": "Current",
                    "previousAvatarName": "Previous",
                    "ownerId": "usr_owner",
                    "previousOwnerId": "usr_previous_owner",
                    "currentAvatarTags": ["content_horror"],
                    "previousCurrentAvatarTags": [],
                })),
            }],
            min_live_sequence: 0,
            max_rows: 10,
        });

        assert_eq!(output.rows.len(), 1);
        let row = &output.rows[0];
        assert_eq!(row.previous_avatar_name.as_deref(), Some("Previous"));
        assert_eq!(row.previous_owner_id.as_deref(), Some("usr_previous_owner"));
        assert_eq!(
            row.current_avatar_tags.as_deref(),
            Some(["content_horror".to_string()].as_slice())
        );
        assert_eq!(
            row.previous_current_avatar_tags.as_deref(),
            Some([].as_slice())
        );
        assert_eq!(row.row_id, None);
    }

    #[test]
    fn live_avatar_search_matches_private_and_public_with_dates_and_filters() {
        let live_entries = vec![
            FeedLiveEntryInput {
                sequence: 1,
                entry: RawJson::from(json!({
                    "type": "Avatar",
                    "userId": "usr_old",
                    "ownerId": "usr_old",
                    "avatarName": "Old",
                    "created_at": "2026-05-01T00:00:00Z",
                })),
            },
            FeedLiveEntryInput {
                sequence: 2,
                entry: RawJson::from(json!({
                    "type": "Avatar",
                    "userId": "usr_private",
                    "ownerId": "usr_private",
                    "avatarName": "Owned Avatar",
                    "created_at": "2026-05-20T00:00:00Z",
                })),
            },
            FeedLiveEntryInput {
                sequence: 3,
                entry: RawJson::from(json!({
                    "type": "Avatar",
                    "userId": "usr_public",
                    "ownerId": "usr_author",
                    "avatarName": "Shared Avatar",
                    "created_at": "2026-05-21T00:00:00Z",
                })),
            },
            FeedLiveEntryInput {
                sequence: 4,
                entry: RawJson::from(json!({
                    "type": "Avatar",
                    "userId": "usr_missing_owner",
                    "avatarName": "Missing Owner",
                    "created_at": "2026-05-21T00:00:00Z",
                })),
            },
            FeedLiveEntryInput {
                sequence: 5,
                entry: RawJson::from(json!({
                    "type": "GPS",
                    "userId": "usr_gps",
                    "ownerId": "usr_other",
                    "created_at": "2026-05-20T00:00:00Z",
                })),
            },
        ];

        let private_output = merge_case(MergeCase {
            filters: vec![FeedFilter::Avatar],
            search: "private".into(),
            date_from: "2026-05-10T00:00:00Z".into(),
            date_to: "2026-05-20T00:00:00Z".into(),
            live_entries: live_entries.clone(),
            max_rows: 10,
            ..MergeCase::default()
        });
        assert_eq!(private_output.rows.len(), 1);
        assert_eq!(
            private_output.rows[0].user_id.as_deref(),
            Some("usr_private")
        );

        let public_output = merge_case(MergeCase {
            filters: vec![FeedFilter::Avatar],
            search: "public".into(),
            date_from: "2026-05-21T00:00:00Z".into(),
            date_to: "2026-05-21T00:00:00Z".into(),
            live_entries,
            max_rows: 10,
            ..MergeCase::default()
        });
        assert_eq!(public_output.rows.len(), 1);
        assert_eq!(public_output.rows[0].user_id.as_deref(), Some("usr_public"));
    }

    #[test]
    fn merged_rows_normalize_snake_case_live_entry_field_names() {
        let output = merge_case(MergeCase {
            rows: Vec::new(),
            current_user_id: "usr_self".into(),
            filters: Vec::new(),
            search: String::new(),
            date_from: String::new(),
            date_to: String::new(),
            favorites_only: false,
            favorite_user_ids: Vec::new(),
            scoped_user_ids: Vec::new(),
            excluded_user_ids: Vec::new(),
            live_entries: vec![FeedLiveEntryInput {
                sequence: 1,
                entry: RawJson::from(json!({
                    "type": "GPS",
                    "user_id": "usr_friend",
                    "display_name": "Friend",
                    "createdAt": "2026-05-15T00:00:00Z",
                    "location": "wrld_1:instance",
                    "world_name": "World",
                    "time": "1500",
                })),
            }],
            min_live_sequence: 0,
            max_rows: 10,
        });

        assert_eq!(output.rows.len(), 1);
        let row = &output.rows[0];
        assert_eq!(row.user_id.as_deref(), Some("usr_friend"));
        assert_eq!(row.display_name.as_deref(), Some("Friend"));
        assert_eq!(row.created_at.as_deref(), Some("2026-05-15T00:00:00Z"));
        assert_eq!(row.world_name.as_deref(), Some("World"));
        assert_eq!(row.time, Some(1500));
    }

    #[test]
    fn persisted_rows_with_matching_content_identity_remain_distinct() {
        let output = merge_case(MergeCase {
            rows: vec![
                RawJson::from(json!({
                    "rowId": 1,
                    "sourceRank": 40,
                    "type": "Status",
                    "userId": "usr_friend",
                    "created_at": "2026-05-15T00:00:00Z",
                    "status": "active"
                })),
                RawJson::from(json!({
                    "rowId": 2,
                    "sourceRank": 40,
                    "type": "Status",
                    "userId": "usr_friend",
                    "created_at": "2026-05-15T00:00:00Z",
                    "status": "join me"
                })),
            ],
            current_user_id: "usr_self".into(),
            max_rows: 10,
            ..MergeCase::default()
        });

        assert_eq!(output.rows.len(), 2);
        assert_eq!(output.rows[0].row_id, Some(1));
        assert_eq!(output.rows[1].row_id, Some(2));
    }

    #[test]
    fn live_row_replaces_the_same_persisted_feed_entry() {
        let output = merge_case(MergeCase {
            rows: vec![RawJson::from(json!({
                "rowId": 1,
                "sourceRank": 60,
                "type": "GPS",
                "userId": "usr_friend",
                "created_at": "2026-05-15T00:00:00Z",
                "location": "wrld_1:instance"
            }))],
            current_user_id: "usr_self".into(),
            live_entries: vec![FeedLiveEntryInput {
                sequence: 1,
                entry: RawJson::from(json!({
                    "type": "GPS",
                    "userId": "usr_friend",
                    "created_at": "2026-05-15T00:00:00Z",
                    "location": "wrld_1:instance"
                })),
            }],
            max_rows: 10,
            ..MergeCase::default()
        });

        assert_eq!(output.rows.len(), 1);
        assert_eq!(output.rows[0].row_id, None);
        assert_eq!(output.max_sequence, 1);
    }

    #[test]
    fn latest_query_keeps_the_persisted_cursor_when_live_rows_fill_the_result(
    ) -> Result<(), crate::Error> {
        let dir = TestDir::new("feed-latest-persisted-cursor");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;
        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                feed_entries: vec![json!({
                    "created_at": "2026-05-15T00:00:00Z",
                    "type": "GPS",
                    "userId": "usr_persisted",
                    "displayName": "Persisted",
                    "location": "wrld_1:persisted",
                    "worldName": "Persisted World",
                    "previousLocation": "",
                    "time": 0,
                    "groupName": ""
                })],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let output = feed_latest_query(
            &db,
            FeedLatestQueryInput {
                user_id: "usr_self".into(),
                filters: vec![FeedFilter::Gps],
                favorite_user_ids: Vec::new(),
                scoped_user_ids: Vec::new(),
                excluded_user_ids: Vec::new(),
                favorites_only: false,
                max_rows: 1,
            },
            vec![FeedLiveEntryInput {
                sequence: 1,
                entry: RawJson::from(json!({
                    "type": "GPS",
                    "userId": "usr_live",
                    "created_at": "2026-05-15T00:01:00Z",
                    "location": "wrld_1:live"
                })),
            }],
            1,
            true,
        )?;

        assert_eq!(output.rows.len(), 1);
        assert_eq!(output.rows[0].user_id.as_deref(), Some("usr_live"));
        assert_eq!(output.rows[0].row_id, None);
        assert!(output.persisted_has_more);
        assert!(output.persisted_cursor.is_some());
        Ok(())
    }

    #[test]
    fn lookup_feed_pagination_uses_the_same_date_order_as_its_cursor() -> Result<(), crate::Error> {
        let dir = TestDir::new("feed-lookup-rowid");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;

        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                feed_entries: vec![
                    json!({
                        "created_at": "2026-05-15T00:10:00Z",
                        "type": "GPS",
                        "userId": "usr_newer_created",
                        "displayName": "newer-created",
                        "location": "wrld_1:newer",
                        "worldName": "Newer Created",
                        "previousLocation": "",
                        "time": 0,
                        "groupName": ""
                    }),
                    json!({
                        "created_at": "2026-05-15T00:00:00Z",
                        "type": "GPS",
                        "userId": "usr_later_inserted",
                        "displayName": "later-inserted",
                        "location": "wrld_1:later",
                        "worldName": "Later Inserted",
                        "previousLocation": "",
                        "time": 0,
                        "groupName": ""
                    }),
                ],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let first_page = feed_rows_query(
            &db,
            FeedRowsQueryInput {
                user_id: "usr_self".into(),
                mode: FeedQueryMode::Lookup,
                search: String::new(),
                filters: vec![FeedFilter::Gps],
                vip_list: Vec::new(),
                scoped_user_ids: Vec::new(),
                excluded_user_ids: Vec::new(),
                max_entries: 1,
                date_from: String::new(),
                date_to: String::new(),
                cursor: None,
            },
        )?;

        assert_eq!(first_page.len(), 1);
        assert_eq!(first_page[0].display_name.as_deref(), Some("newer-created"));

        let second_page = feed_rows_query(
            &db,
            FeedRowsQueryInput {
                user_id: "usr_self".into(),
                mode: FeedQueryMode::Lookup,
                search: String::new(),
                filters: vec![FeedFilter::Gps],
                vip_list: Vec::new(),
                scoped_user_ids: Vec::new(),
                excluded_user_ids: Vec::new(),
                max_entries: 1,
                date_from: String::new(),
                date_to: String::new(),
                cursor: Some(FeedCursorInput {
                    created_at: first_page[0].created_at.clone().unwrap(),
                    source_rank: first_page[0].source_rank.unwrap(),
                    row_id: first_page[0].row_id.unwrap(),
                }),
            },
        )?;

        assert_eq!(second_page.len(), 1);
        assert_eq!(
            second_page[0].display_name.as_deref(),
            Some("later-inserted")
        );
        Ok(())
    }

    #[test]
    fn world_id_search_honors_the_date_window() -> Result<(), crate::Error> {
        let dir = TestDir::new("feed-search-world-date-window");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;
        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                feed_entries: vec![
                    json!({
                        "created_at": "2026-05-01T00:00:00Z",
                        "type": "GPS",
                        "userId": "usr_old",
                        "displayName": "Old",
                        "location": "wrld_target:old",
                        "worldName": "Target",
                    }),
                    json!({
                        "created_at": "2026-05-20T00:00:00Z",
                        "type": "GPS",
                        "userId": "usr_new",
                        "displayName": "New",
                        "location": "wrld_target:new",
                        "worldName": "Target",
                    }),
                ],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let rows = feed_rows_query_interruptible(
            &db,
            FeedRowsQueryInput {
                user_id: "usr_self".into(),
                mode: FeedQueryMode::Search,
                search: "wrld_target".into(),
                filters: vec![FeedFilter::Gps],
                vip_list: Vec::new(),
                scoped_user_ids: Vec::new(),
                excluded_user_ids: Vec::new(),
                max_entries: 10,
                date_from: "2026-05-10T00:00:00Z".into(),
                date_to: String::new(),
                cursor: None,
            },
            || false,
        )?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].user_id.as_deref(), Some("usr_new"));
        Ok(())
    }

    #[test]
    fn private_avatar_search_applies_dates_to_every_match_branch() -> Result<(), crate::Error> {
        let dir = TestDir::new("feed-search-private-avatar-date-window");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;
        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                feed_entries: vec![
                    json!({
                        "created_at": "2026-05-01T00:00:00Z",
                        "type": "Avatar",
                        "userId": "usr_old",
                        "displayName": "Private Collector",
                        "ownerId": "usr_old",
                        "avatarName": "Old Avatar",
                    }),
                    json!({
                        "created_at": "2026-05-20T00:00:00Z",
                        "type": "Avatar",
                        "userId": "usr_new",
                        "displayName": "New",
                        "ownerId": "usr_new",
                        "avatarName": "New Avatar",
                    }),
                ],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let rows = feed_rows_query_interruptible(
            &db,
            FeedRowsQueryInput {
                user_id: "usr_self".into(),
                mode: FeedQueryMode::Search,
                search: "private".into(),
                filters: vec![FeedFilter::Avatar],
                vip_list: Vec::new(),
                scoped_user_ids: Vec::new(),
                excluded_user_ids: Vec::new(),
                max_entries: 10,
                date_from: "2026-05-10T00:00:00Z".into(),
                date_to: String::new(),
                cursor: None,
            },
            || false,
        )?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].user_id.as_deref(), Some("usr_new"));
        Ok(())
    }

    #[test]
    fn date_window_preserves_millisecond_boundaries() -> Result<(), crate::Error> {
        let dir = TestDir::new("feed-millisecond-date-window");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;
        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                feed_entries: vec![
                    json!({
                        "created_at": "2026-05-20T00:00:00.000Z",
                        "type": "Status",
                        "userId": "usr_milliseconds",
                        "displayName": "Milliseconds",
                        "status": "active",
                    }),
                    json!({
                        "created_at": "2026-05-20T00:00:00Z",
                        "type": "Status",
                        "userId": "usr_seconds",
                        "displayName": "Seconds",
                        "status": "active",
                    }),
                    json!({
                        "created_at": "2026-05-20T00:00:00+00:00",
                        "type": "Status",
                        "userId": "usr_offset",
                        "displayName": "Offset",
                        "status": "active",
                    }),
                    json!({
                        "created_at": "2026-05-20T00:00:00.500Z",
                        "type": "Status",
                        "userId": "usr_later",
                        "displayName": "Later",
                        "status": "active",
                    }),
                ],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let rows = feed_rows_query_interruptible(
            &db,
            FeedRowsQueryInput {
                user_id: "usr_self".into(),
                mode: FeedQueryMode::Lookup,
                search: String::new(),
                filters: vec![FeedFilter::Status],
                vip_list: Vec::new(),
                scoped_user_ids: Vec::new(),
                excluded_user_ids: Vec::new(),
                max_entries: 10,
                date_from: "2026-05-20T00:00:00.000Z".into(),
                date_to: "2026-05-20T00:00:00.000Z".into(),
                cursor: None,
            },
            || false,
        )?;

        assert_eq!(rows.len(), 3);
        assert!(rows
            .iter()
            .all(|row| row.user_id.as_deref() != Some("usr_later")));

        let newest = feed_rows_query_interruptible(
            &db,
            FeedRowsQueryInput {
                user_id: "usr_self".into(),
                mode: FeedQueryMode::Lookup,
                search: String::new(),
                filters: vec![FeedFilter::Status],
                vip_list: Vec::new(),
                scoped_user_ids: Vec::new(),
                excluded_user_ids: Vec::new(),
                max_entries: 1,
                date_from: String::new(),
                date_to: String::new(),
                cursor: None,
            },
            || false,
        )?;
        assert_eq!(newest[0].user_id.as_deref(), Some("usr_later"));
        Ok(())
    }

    #[test]
    fn search_matches_previous_values_and_escapes_like_wildcards() -> Result<(), crate::Error> {
        let dir = TestDir::new("feed-search-previous-literal");
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;
        write_realtime_batch(
            &db,
            "usr_self",
            &RealtimePersistenceBatch {
                feed_entries: vec![
                    json!({
                        "created_at": "2026-05-20T00:00:00.000Z",
                        "type": "Bio",
                        "userId": "usr_previous",
                        "displayName": "Previous",
                        "bio": "new value",
                        "previousBio": "removed needle",
                    }),
                    json!({
                        "created_at": "2026-05-19T00:00:00.000Z",
                        "type": "Bio",
                        "userId": "usr_percent",
                        "displayName": "Percent",
                        "bio": "100% literal",
                        "previousBio": "old",
                    }),
                    json!({
                        "created_at": "2026-05-18T00:00:00.000Z",
                        "type": "Bio",
                        "userId": "usr_other",
                        "displayName": "Other",
                        "bio": "ordinary text",
                        "previousBio": "old",
                    }),
                ],
                ..RealtimePersistenceBatch::default()
            },
        )?;

        let search = |text: &str| {
            feed_rows_query(
                &db,
                FeedRowsQueryInput {
                    user_id: "usr_self".into(),
                    mode: FeedQueryMode::Search,
                    search: text.into(),
                    filters: vec![FeedFilter::Bio],
                    vip_list: Vec::new(),
                    scoped_user_ids: Vec::new(),
                    excluded_user_ids: Vec::new(),
                    max_entries: 10,
                    date_from: String::new(),
                    date_to: String::new(),
                    cursor: None,
                },
            )
        };

        let previous = search("removed needle")?;
        assert_eq!(previous.len(), 1);
        assert_eq!(previous[0].user_id.as_deref(), Some("usr_previous"));

        let literal_percent = search("%")?;
        assert_eq!(literal_percent.len(), 1);
        assert_eq!(literal_percent[0].user_id.as_deref(), Some("usr_percent"));
        Ok(())
    }
}
