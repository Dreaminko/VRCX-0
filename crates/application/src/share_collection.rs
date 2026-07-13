use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vrcx_0_integrations::world_collections::{
    create_world_collection, WorldCollectionCreatePayload, WorldCollectionPayloadWorld,
    WORLD_COLLECTIONS_SITE_ORIGIN,
};
use vrcx_0_persistence::{
    config::{get_json, set_json},
    memos::memo_get_worlds_many,
    worlds::{world_cache_get_many, WorldSummaryOutput},
    DatabaseService,
};

use crate::Error;

pub const SHARE_COLLECTION_MAX_WORLDS: usize = 1_000;
const SHARE_COLLECTION_WORLD_BATCH_SIZE: usize = 500;
const SHARE_OWNER_KEYS_CONFIG_KEY: &str = "VRCX_ShareOwnerKeys";
static SHARE_OWNER_KEYS_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShareCollectionCreateInput {
    pub title: String,
    pub listed: bool,
    pub include_notes: bool,
    pub world_ids: Vec<String>,
}

pub struct ShareCollectionDeps<'a> {
    pub db: &'a DatabaseService,
    pub current_user_id: &'a str,
    pub current_user_display_name: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedShareCollection {
    pub payload: WorldCollectionCreatePayload,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShareCollectionCreateResult {
    pub id: String,
    pub url: String,
    pub world_count: i64,
}

pub fn prepare_share_collection_payload(
    deps: ShareCollectionDeps<'_>,
    input: ShareCollectionCreateInput,
) -> Result<PreparedShareCollection, Error> {
    let title = normalize_title(&input.title)?;
    let owner_key = get_or_create_share_owner_key(deps.db, deps.current_user_id)?;
    let owner_hint = share_collection_owner_hint(deps.current_user_id);
    let author_name = deps.current_user_display_name.trim().to_string();
    let normalized_world_ids = normalize_world_ids(&input.world_ids);
    let truncated = normalized_world_ids.len() > SHARE_COLLECTION_MAX_WORLDS;
    let capped_world_ids = normalized_world_ids
        .into_iter()
        .take(SHARE_COLLECTION_MAX_WORLDS)
        .collect::<Vec<_>>();
    if capped_world_ids.is_empty() {
        return Err(Error::Custom(
            "Share collection requires at least one world id.".into(),
        ));
    }

    let mut rows = Vec::new();
    for world_id_batch in capped_world_ids.chunks(SHARE_COLLECTION_WORLD_BATCH_SIZE) {
        rows.extend(world_cache_get_many(deps.db, world_id_batch)?);
    }
    let rows_by_id = rows
        .into_iter()
        .map(|row| (row.id.clone(), row))
        .collect::<HashMap<_, _>>();

    let mut memos_by_id: HashMap<String, String> = HashMap::new();
    if input.include_notes {
        for world_id_batch in capped_world_ids.chunks(SHARE_COLLECTION_WORLD_BATCH_SIZE) {
            for memo in memo_get_worlds_many(deps.db, world_id_batch)? {
                memos_by_id.insert(memo.world_id, memo.memo);
            }
        }
    }

    let mut worlds = Vec::new();
    for world_id in capped_world_ids {
        let Some(row) = rows_by_id.get(&world_id) else {
            continue;
        };
        if !row.release_status.eq_ignore_ascii_case("public") {
            continue;
        }
        worlds.push(payload_world_from_row(row, &memos_by_id));
    }
    if worlds.is_empty() {
        return Err(Error::Custom(
            "Share collection has no public cached worlds to upload.".into(),
        ));
    }

    Ok(PreparedShareCollection {
        payload: WorldCollectionCreatePayload {
            schema: 1,
            owner_key,
            owner_hint,
            title,
            listed: input.listed,
            access: "open".into(),
            author_name,
            updated_at: Utc::now().timestamp(),
            worlds,
        },
        truncated,
    })
}

pub async fn share_collection_create(
    deps: ShareCollectionDeps<'_>,
    input: ShareCollectionCreateInput,
) -> Result<ShareCollectionCreateResult, Error> {
    let prepared = prepare_share_collection_payload(deps, input)?;
    let world_count = prepared.payload.worlds.len() as i64;
    let response = create_world_collection(&prepared.payload)
        .await
        .map_err(|error| Error::Custom(error.to_string()))?;
    let id = response.id;
    let url = format!("{WORLD_COLLECTIONS_SITE_ORIGIN}/c/{id}");
    Ok(ShareCollectionCreateResult {
        id,
        url,
        world_count,
    })
}

fn payload_world_from_row(
    row: &WorldSummaryOutput,
    memos_by_id: &HashMap<String, String>,
) -> WorldCollectionPayloadWorld {
    let comment = memos_by_id.get(&row.id).cloned().unwrap_or_default();
    let image_url = if row.image_url.trim().is_empty() {
        row.thumbnail_image_url.clone()
    } else {
        row.image_url.clone()
    };
    WorldCollectionPayloadWorld {
        world_id: row.id.clone(),
        author_id: row.author_id.clone(),
        name: row.name.clone(),
        author_name: row.author_name.clone(),
        created_at: row.created_at.clone(),
        image_url,
        description: row.description.clone(),
        release_status: row.release_status.clone(),
        thumbnail_image_url: row.thumbnail_image_url.clone(),
        comment,
        updated_at: row.updated_at.clone(),
        version: row.version,
    }
}

pub fn get_or_create_share_owner_key(db: &DatabaseService, user_id: &str) -> Result<String, Error> {
    let user_id = user_id.trim();
    if user_id.is_empty() {
        return Err(Error::Custom(
            "Share collection requires an authenticated user.".into(),
        ));
    }

    let _guard = SHARE_OWNER_KEYS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut owner_keys = read_share_owner_keys(db)?;
    if let Some(owner_key) = owner_keys.get(user_id) {
        return Ok(owner_key.clone());
    }

    let owner_key = generate_share_owner_key()?;
    owner_keys.insert(user_id.to_string(), owner_key.clone());
    write_share_owner_keys(db, &owner_keys)?;
    Ok(owner_key)
}

pub fn share_collection_owner_hint(user_id: &str) -> String {
    hex::encode(Sha256::digest(user_id.trim().as_bytes()))
}

fn generate_share_owner_key() -> Result<String, Error> {
    let mut owner_key_bytes = [0_u8; 32];
    getrandom::fill(&mut owner_key_bytes)
        .map_err(|error| Error::Custom(format!("failed to generate share owner key: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(owner_key_bytes))
}

fn read_share_owner_keys(db: &DatabaseService) -> Result<HashMap<String, String>, Error> {
    let raw = get_json(db, SHARE_OWNER_KEYS_CONFIG_KEY, serde_json::json!({}))?;
    Ok(serde_json::from_value(raw).unwrap_or_default())
}

fn write_share_owner_keys(
    db: &DatabaseService,
    owner_keys: &HashMap<String, String>,
) -> Result<(), Error> {
    let value = serde_json::to_value(owner_keys)?;
    set_json(db, SHARE_OWNER_KEYS_CONFIG_KEY, &value)?;
    Ok(())
}

fn normalize_title(title: &str) -> Result<String, Error> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Custom("Share collection title is required.".into()));
    }
    Ok(title.to_string())
}

fn normalize_world_ids(world_ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for world_id in world_ids {
        let world_id = world_id.trim();
        if !world_id.starts_with("wrld_") {
            continue;
        }
        if !seen.insert(world_id) {
            continue;
        }
        normalized.push(world_id.to_string());
    }
    normalized
}
