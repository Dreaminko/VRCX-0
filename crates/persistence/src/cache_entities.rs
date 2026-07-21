use serde::Deserialize;
use serde_json::Value;

use crate::common::{now_iso, ParamsBuilder};
use crate::database::schema::ensure_global_store_tables;
use crate::database::DatabaseService;
use crate::Error;

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntityInput {
    #[serde(default)]
    pub id: Value,
    #[serde(default)]
    pub author_id: Value,
    #[serde(default)]
    pub author_name: Value,
    #[serde(default)]
    pub created_at: Value,
    #[serde(default)]
    pub description: Value,
    #[serde(default)]
    pub image_url: Value,
    #[serde(default)]
    pub name: Value,
    #[serde(default)]
    pub release_status: Value,
    #[serde(default)]
    pub thumbnail_image_url: Value,
    #[serde(default)]
    pub updated_at: Value,
    #[serde(default)]
    pub version: Value,
}

pub(crate) fn upsert_cache_entity(
    db: &DatabaseService,
    table_name: &str,
    entry: CacheEntityInput,
) -> Result<i64, Error> {
    let id = entry
        .id
        .as_str()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| Error::InvalidData("Cache entity id must be a non-empty string.".into()))?
        .to_owned();
    ensure_global_store_tables(db)?;
    let now = now_iso();
    db.execute_non_query(
        &format!("INSERT OR REPLACE INTO {table_name} (id, added_at, author_id, author_name, created_at, description, image_url, name, release_status, thumbnail_image_url, updated_at, version) VALUES (@id, @added_at, @author_id, @author_name, @created_at, @description, @image_url, @name, @release_status, @thumbnail_image_url, @updated_at, @version)"),
        &ParamsBuilder::new()
            .set("id", id)
            .set("added_at", now)
            .set("author_id", entry.author_id)
            .set("author_name", entry.author_name)
            .set("created_at", entry.created_at)
            .set("description", entry.description)
            .set("image_url", entry.image_url)
            .set("name", entry.name)
            .set("release_status", entry.release_status)
            .set("thumbnail_image_url", entry.thumbnail_image_url)
            .set("updated_at", entry.updated_at)
            .set("version", entry.version)
            .build(),
    )
}
