use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use vrcx_0_core::screenshots::{
    ScreenshotFolderInfo, ScreenshotFolderTree, ScreenshotLibraryImage,
    ScreenshotLibraryScanStatus, ScreenshotMetadata,
};

use crate::{Error, Result};

pub const SCREENSHOT_LIBRARY_INDEX_VERSION: i64 = 1;

#[derive(Clone, Debug)]
pub struct ScreenshotLibraryEntry {
    pub scan_root: String,
    pub path: String,
    pub folder_path: String,
    pub file_name: String,
    pub size_bytes: i64,
    pub modified_at: i64,
    pub created_at: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub world_id: Option<String>,
    pub world_name: Option<String>,
    pub captured_at: Option<String>,
    pub metadata_json: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct ScreenshotLibraryCachedState {
    pub size_bytes: i64,
    pub modified_at: i64,
    pub index_version: i64,
}

pub struct ScreenshotThumbnailCacheEntry {
    pub thumb_path: String,
    pub source_path: String,
    pub cache_key: String,
    pub size_bytes: i64,
    pub modified_at: i64,
    pub last_used_at: i64,
}

#[derive(Clone)]
pub struct MetadataCacheDb {
    conn: Arc<Mutex<Connection>>,
    scan_status: Arc<Mutex<ScreenshotLibraryScanStatus>>,
    scan_running: Arc<AtomicBool>,
}

impl MetadataCacheDb {
    pub fn new(db_path: &Path) -> Result<Self> {
        let mut conn = Connection::open(db_path)
            .map_err(|e| Error::Database(format!("open cache db: {e}")))?;
        conn.execute_batch(
            "PRAGMA locking_mode=NORMAL;
             PRAGMA busy_timeout=5000;
             PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS cache (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 file_path TEXT NOT NULL UNIQUE,
                 metadata TEXT,
                 cached_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS screenshot_files (
                 path TEXT PRIMARY KEY,
                 scan_root TEXT NOT NULL DEFAULT '',
                 folder_path TEXT NOT NULL,
                 file_name TEXT NOT NULL,
                 size_bytes INTEGER NOT NULL,
                 modified_at INTEGER NOT NULL,
                 created_at INTEGER,
                 width INTEGER,
                 height INTEGER,
                 world_id TEXT,
                 world_name TEXT,
                 captured_at TEXT,
                 metadata_json TEXT,
                 index_version INTEGER NOT NULL DEFAULT 0,
                 indexed_at INTEGER NOT NULL,
                 error TEXT
             );
             CREATE TABLE IF NOT EXISTS screenshot_thumbnail_cache (
                 thumb_path TEXT PRIMARY KEY,
                 source_path TEXT NOT NULL,
                 cache_key TEXT NOT NULL,
                 size_bytes INTEGER NOT NULL,
                 modified_at INTEGER NOT NULL,
                 created_at INTEGER NOT NULL,
                 last_used_at INTEGER NOT NULL
             );",
        )
        .map_err(|e| Error::Database(format!("init cache db: {e}")))?;
        let _ = conn.execute(
            "ALTER TABLE screenshot_files ADD COLUMN scan_root TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE screenshot_thumbnail_cache ADD COLUMN cache_key TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE screenshot_files ADD COLUMN index_version INTEGER NOT NULL DEFAULT 0",
            [],
        );
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_screenshot_files_folder_path
                 ON screenshot_files(scan_root, folder_path);
             CREATE INDEX IF NOT EXISTS idx_screenshot_files_world_id
                 ON screenshot_files(scan_root, world_id);
             CREATE INDEX IF NOT EXISTS idx_screenshot_files_modified_at
                 ON screenshot_files(scan_root, modified_at);
             CREATE INDEX IF NOT EXISTS idx_screenshot_thumbnail_cache_source
                 ON screenshot_thumbnail_cache(source_path);",
        )
        .map_err(|e| Error::Database(format!("init screenshot db indexes: {e}")))?;
        let thumbnail_dir = db_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join("ScreenshotThumbs");
        normalize_thumbnail_cache_paths(&mut conn, &thumbnail_dir)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            scan_status: Arc::new(Mutex::new(ScreenshotLibraryScanStatus::default())),
            scan_running: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn is_cached(&self, file_path: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT 1 FROM cache WHERE file_path = ?1 LIMIT 1",
            [file_path],
            |_| Ok(()),
        )
        .is_ok()
    }

    pub fn get_metadata(&self, file_path: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT metadata FROM cache WHERE file_path = ?1 LIMIT 1",
            [file_path],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
    }

    pub fn bulk_add(&self, entries: &[(String, Option<String>)]) {
        let conn = self.conn.lock().unwrap();
        let tx = match conn.unchecked_transaction() {
            Ok(t) => t,
            Err(_) => return,
        };
        {
            let mut stmt = match tx.prepare(
                "INSERT OR IGNORE INTO cache (file_path, metadata, cached_at) VALUES (?1, ?2, ?3)",
            ) {
                Ok(s) => s,
                Err(_) => return,
            };
            let now = now_unix_seconds();
            for (path, meta) in entries {
                let _ = stmt.execute(rusqlite::params![path, meta.as_deref(), now]);
            }
        }
        let _ = tx.commit();
    }

    pub fn scan_status(&self) -> ScreenshotLibraryScanStatus {
        self.scan_status.lock().unwrap().clone()
    }

    pub fn set_scan_status(&self, status: ScreenshotLibraryScanStatus) {
        *self.scan_status.lock().unwrap() = status;
    }

    pub fn try_begin_scan(&self) -> bool {
        self.scan_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn finish_scan(&self, status: ScreenshotLibraryScanStatus) {
        self.set_scan_status(status);
        self.scan_running.store(false, Ordering::SeqCst);
    }

    pub fn library_file_states(&self, root: &str) -> HashMap<String, ScreenshotLibraryCachedState> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT path, size_bytes, modified_at, index_version
             FROM screenshot_files
             WHERE scan_root = ?1",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return HashMap::new(),
        };
        let rows = match stmt.query_map([root], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        }) {
            Ok(rows) => rows,
            Err(_) => return HashMap::new(),
        };
        rows.filter_map(|row| row.ok())
            .map(|(path, size_bytes, modified_at, index_version)| {
                (
                    path,
                    ScreenshotLibraryCachedState {
                        size_bytes,
                        modified_at,
                        index_version,
                    },
                )
            })
            .collect()
    }

    pub fn replace_library_entries(
        &self,
        root: &str,
        seen: &HashSet<String>,
        entries: &[ScreenshotLibraryEntry],
        prune_missing: bool,
    ) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(|error| {
            Error::Database(format!("start screenshot index transaction: {error}"))
        })?;
        let now = now_unix_seconds();

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO screenshot_files (
                    path, scan_root, folder_path, file_name, size_bytes, modified_at, created_at,
                    width, height, world_id, world_name, captured_at, metadata_json,
                    index_version, indexed_at, error
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                 ON CONFLICT(path) DO UPDATE SET
                    scan_root = excluded.scan_root,
                    folder_path = excluded.folder_path,
                    file_name = excluded.file_name,
                    size_bytes = excluded.size_bytes,
                    modified_at = excluded.modified_at,
                    created_at = excluded.created_at,
                    width = excluded.width,
                    height = excluded.height,
                    world_id = excluded.world_id,
                    world_name = excluded.world_name,
                    captured_at = excluded.captured_at,
                    metadata_json = excluded.metadata_json,
                    index_version = excluded.index_version,
                    indexed_at = excluded.indexed_at,
                    error = excluded.error",
                )
                .map_err(|error| {
                    Error::Database(format!("prepare screenshot index upsert: {error}"))
                })?;

            for entry in entries {
                stmt.execute(rusqlite::params![
                    entry.path.as_str(),
                    entry.scan_root.as_str(),
                    entry.folder_path.as_str(),
                    entry.file_name.as_str(),
                    entry.size_bytes,
                    entry.modified_at,
                    entry.created_at,
                    entry.width,
                    entry.height,
                    entry.world_id.as_deref(),
                    entry.world_name.as_deref(),
                    entry.captured_at.as_deref(),
                    entry.metadata_json.as_deref(),
                    SCREENSHOT_LIBRARY_INDEX_VERSION,
                    now,
                    entry.error.as_deref(),
                ])
                .map_err(|error| Error::Database(format!("write screenshot index row: {error}")))?;
            }
        }

        let mut deleted = 0;
        if prune_missing {
            let existing_paths = {
                let mut stmt = tx
                    .prepare("SELECT path FROM screenshot_files WHERE scan_root = ?1")
                    .map_err(|error| {
                        Error::Database(format!("prepare screenshot index prune: {error}"))
                    })?;
                let rows = stmt
                    .query_map([root], |row| row.get::<_, String>(0))
                    .map_err(|error| {
                        Error::Database(format!("read screenshot index prune set: {error}"))
                    })?;
                rows.filter_map(|row| row.ok()).collect::<Vec<_>>()
            };

            for path in existing_paths {
                if !seen.contains(&path) {
                    tx.execute("DELETE FROM screenshot_files WHERE path = ?1", [&path])
                        .map_err(|error| {
                            Error::Database(format!("delete stale screenshot index row: {error}"))
                        })?;
                    deleted += 1;
                }
            }
        }

        tx.commit().map_err(|error| {
            Error::Database(format!("commit screenshot index transaction: {error}"))
        })?;
        Ok(deleted)
    }

    #[doc(hidden)]
    pub fn mark_library_entry_stale_for_test(&self, path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE screenshot_files SET index_version = 0, metadata_json = NULL WHERE path = ?1",
            [path],
        )
        .map_err(|error| Error::Database(format!("mark screenshot row stale: {error}")))?;
        Ok(())
    }

    pub fn screenshot_folder_tree_for_root(&self, root_path: &str) -> Result<ScreenshotFolderTree> {
        let conn = self.conn.lock().unwrap();
        let mut direct_counts: HashMap<String, usize> = HashMap::new();
        let mut latest_modified_by_folder: HashMap<String, i64> = HashMap::new();
        let mut stmt = conn
            .prepare(
                "SELECT folder_path, COUNT(*), MAX(modified_at)
             FROM screenshot_files
             WHERE scan_root = ?1
             GROUP BY folder_path",
            )
            .map_err(|error| Error::Database(format!("prepare screenshot folder tree: {error}")))?;
        let rows = stmt
            .query_map([root_path], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })
            .map_err(|error| Error::Database(format!("read screenshot folder tree: {error}")))?;
        for row in rows {
            let (folder_path, count, latest_modified_at) = row
                .map_err(|error| Error::Database(format!("read screenshot folder row: {error}")))?;
            if let Some(latest_modified_at) = latest_modified_at {
                latest_modified_by_folder.insert(folder_path.clone(), latest_modified_at);
            }
            direct_counts.insert(folder_path, count.max(0) as usize);
        }

        if root_path.is_empty() {
            return Ok(ScreenshotFolderTree {
                root_path: root_path.to_string(),
                folders: Vec::new(),
            });
        }

        let root = PathBuf::from(root_path);
        let mut folder_paths = HashSet::new();
        folder_paths.insert(root_path.to_string());
        for folder in direct_counts.keys() {
            let mut current = PathBuf::from(folder);
            loop {
                folder_paths.insert(path_string(&current));
                if current == root {
                    break;
                }
                let Some(parent) = current.parent() else {
                    break;
                };
                current = parent.to_path_buf();
            }
        }

        let mut children_by_parent: HashMap<String, Vec<String>> = HashMap::new();
        for folder in &folder_paths {
            let path = PathBuf::from(folder);
            let parent_path = path.parent().map(path_string);
            if let Some(parent_path) = parent_path {
                if folder_paths.contains(&parent_path) {
                    children_by_parent
                        .entry(parent_path)
                        .or_default()
                        .push(folder.clone());
                }
            }
        }

        fn total_count(
            path: &str,
            direct_counts: &HashMap<String, usize>,
            children_by_parent: &HashMap<String, Vec<String>>,
        ) -> usize {
            let own = direct_counts.get(path).copied().unwrap_or(0);
            own + children_by_parent
                .get(path)
                .into_iter()
                .flatten()
                .map(|child| total_count(child, direct_counts, children_by_parent))
                .sum::<usize>()
        }

        let mut folders: Vec<ScreenshotFolderInfo> = folder_paths
            .into_iter()
            .map(|folder| {
                let path = PathBuf::from(&folder);
                let parent_path = path.parent().map(path_string).filter(|parent| {
                    parent == root_path || children_by_parent.contains_key(parent)
                });
                let name = if folder == root_path {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .filter(|name| !name.is_empty())
                        .unwrap_or_else(|| folder.clone())
                } else {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| folder.clone())
                };
                ScreenshotFolderInfo {
                    latest_modified_at: latest_modified_by_folder.get(&folder).copied(),
                    image_count: direct_counts.get(&folder).copied().unwrap_or(0),
                    total_image_count: total_count(&folder, &direct_counts, &children_by_parent),
                    path: folder,
                    parent_path,
                    name,
                }
            })
            .collect();
        folders.sort_by(|left, right| {
            left.path
                .to_lowercase()
                .cmp(&right.path.to_lowercase())
                .then_with(|| left.path.cmp(&right.path))
        });

        Ok(ScreenshotFolderTree {
            root_path: root_path.to_string(),
            folders,
        })
    }

    pub fn list_screenshot_folder_images_for_root(
        &self,
        root_path: &str,
        folder_path: &str,
    ) -> Result<Vec<ScreenshotLibraryImage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT path, folder_path, file_name, size_bytes, modified_at, created_at,
                    width, height, world_id, world_name, captured_at, error, metadata_json
             FROM screenshot_files
             WHERE scan_root = ?1 AND folder_path = ?2
              ORDER BY file_name ASC, modified_at ASC",
            )
            .map_err(|error| {
                Error::Database(format!("prepare screenshot folder images: {error}"))
            })?;
        let rows = stmt
            .query_map([root_path, folder_path], Self::map_library_image_row)
            .map_err(|error| Error::Database(format!("read screenshot folder images: {error}")))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| Error::Database(format!("read screenshot folder image row: {error}")))
    }

    pub fn list_world_screenshots_for_root(
        &self,
        root_path: &str,
        world_id: &str,
    ) -> Result<Vec<ScreenshotLibraryImage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT path, folder_path, file_name, size_bytes, modified_at, created_at,
                    width, height, world_id, world_name, captured_at, error, metadata_json
             FROM screenshot_files
             WHERE scan_root = ?1 AND world_id = ?2
              ORDER BY file_name ASC, modified_at ASC",
            )
            .map_err(|error| Error::Database(format!("prepare world screenshots: {error}")))?;
        let rows = stmt
            .query_map([root_path, world_id], Self::map_library_image_row)
            .map_err(|error| Error::Database(format!("read world screenshots: {error}")))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| Error::Database(format!("read world screenshot row: {error}")))
    }

    pub fn record_thumbnail_cache(
        &self,
        source_path: &str,
        thumb_path: &str,
        cache_key: &str,
        size_bytes: i64,
        modified_at: i64,
    ) {
        let conn = self.conn.lock().unwrap();
        let now = now_unix_seconds();
        let _ = conn.execute(
            "INSERT INTO screenshot_thumbnail_cache (
                thumb_path, source_path, cache_key, size_bytes, modified_at, created_at, last_used_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(thumb_path) DO UPDATE SET
                source_path = excluded.source_path,
                cache_key = excluded.cache_key,
                size_bytes = excluded.size_bytes,
                modified_at = excluded.modified_at,
                last_used_at = excluded.last_used_at",
            rusqlite::params![thumb_path, source_path, cache_key, size_bytes, modified_at, now],
        );
    }

    pub fn thumbnail_cache_entries(&self) -> Vec<ScreenshotThumbnailCacheEntry> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT thumb_path, source_path, cache_key, size_bytes, modified_at, last_used_at
             FROM screenshot_thumbnail_cache",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };
        let entries = match stmt.query_map([], |row| {
            Ok(ScreenshotThumbnailCacheEntry {
                thumb_path: row.get(0)?,
                source_path: row.get(1)?,
                cache_key: row.get(2)?,
                size_bytes: row.get(3)?,
                modified_at: row.get(4)?,
                last_used_at: row.get(5)?,
            })
        }) {
            Ok(rows) => rows.filter_map(|row| row.ok()).collect(),
            Err(_) => Vec::new(),
        };
        entries
    }

    pub fn thumbnail_cache_entries_for_source(
        &self,
        source_path: &str,
    ) -> Vec<ScreenshotThumbnailCacheEntry> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT thumb_path, source_path, cache_key, size_bytes, modified_at, last_used_at
             FROM screenshot_thumbnail_cache
             WHERE source_path = ?1",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };
        let entries = match stmt.query_map([source_path], |row| {
            Ok(ScreenshotThumbnailCacheEntry {
                thumb_path: row.get(0)?,
                source_path: row.get(1)?,
                cache_key: row.get(2)?,
                size_bytes: row.get(3)?,
                modified_at: row.get(4)?,
                last_used_at: row.get(5)?,
            })
        }) {
            Ok(rows) => rows.filter_map(|row| row.ok()).collect(),
            Err(_) => Vec::new(),
        };
        entries
    }

    pub fn thumbnail_last_used_map(&self) -> HashMap<String, i64> {
        self.thumbnail_cache_entries()
            .into_iter()
            .map(|entry| (entry.thumb_path, entry.last_used_at))
            .collect()
    }

    pub fn delete_thumbnail_cache_record(&self, thumb_path: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM screenshot_thumbnail_cache WHERE thumb_path = ?1",
            [thumb_path],
        );
    }

    pub fn clear_all(&self) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM cache", []);
        let _ = conn.execute("DELETE FROM screenshot_files", []);
        let _ = conn.execute("DELETE FROM screenshot_thumbnail_cache", []);
    }

    fn map_library_image_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScreenshotLibraryImage> {
        let metadata_json = row.get::<_, Option<String>>(12)?;
        let metadata = metadata_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<ScreenshotMetadata>(value).ok());
        Ok(ScreenshotLibraryImage {
            path: row.get(0)?,
            folder_path: row.get(1)?,
            file_name: row.get(2)?,
            size_bytes: row.get(3)?,
            modified_at: row.get(4)?,
            created_at: row.get(5)?,
            width: row.get(6)?,
            height: row.get(7)?,
            world_id: row.get(8)?,
            world_name: row.get(9)?,
            captured_at: row.get(10)?,
            error: row.get(11)?,
            metadata,
        })
    }
}

fn normalize_thumbnail_cache_paths(conn: &mut Connection, thumbnail_dir: &Path) -> Result<()> {
    let absolute_paths = {
        let mut stmt = conn
            .prepare("SELECT thumb_path FROM screenshot_thumbnail_cache")
            .map_err(|error| {
                Error::Database(format!("prepare thumbnail path normalization: {error}"))
            })?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| {
                Error::Database(format!("read thumbnail paths for normalization: {error}"))
            })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| Error::Database(format!("read thumbnail path row: {error}")))?
            .into_iter()
            .filter(|path| Path::new(path).is_absolute())
            .collect::<Vec<_>>()
    };
    if absolute_paths.is_empty() {
        return Ok(());
    }

    let transaction = conn
        .transaction()
        .map_err(|error| Error::Database(format!("begin thumbnail path normalization: {error}")))?;
    for absolute_path in absolute_paths {
        let path = Path::new(&absolute_path);
        let Some(relative_path) = path
            .strip_prefix(thumbnail_dir)
            .ok()
            .filter(|relative| !relative.as_os_str().is_empty())
        else {
            transaction
                .execute(
                    "DELETE FROM screenshot_thumbnail_cache WHERE thumb_path = ?1",
                    [&absolute_path],
                )
                .map_err(|error| {
                    Error::Database(format!("remove stale thumbnail path: {error}"))
                })?;
            continue;
        };
        let relative_path = path_string(relative_path);
        let relative_exists = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM screenshot_thumbnail_cache WHERE thumb_path = ?1
                 )",
                [&relative_path],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|error| {
                Error::Database(format!("check normalized thumbnail path: {error}"))
            })?;
        if relative_exists {
            transaction
                .execute(
                    "UPDATE screenshot_thumbnail_cache
                     SET last_used_at = MAX(
                         last_used_at,
                         (SELECT last_used_at
                          FROM screenshot_thumbnail_cache
                          WHERE thumb_path = ?2)
                     )
                     WHERE thumb_path = ?1",
                    rusqlite::params![relative_path, absolute_path],
                )
                .map_err(|error| {
                    Error::Database(format!("merge normalized thumbnail path: {error}"))
                })?;
            transaction
                .execute(
                    "DELETE FROM screenshot_thumbnail_cache WHERE thumb_path = ?1",
                    [&absolute_path],
                )
                .map_err(|error| {
                    Error::Database(format!("remove duplicate thumbnail path: {error}"))
                })?;
        } else {
            transaction
                .execute(
                    "UPDATE screenshot_thumbnail_cache
                     SET thumb_path = ?1
                     WHERE thumb_path = ?2",
                    rusqlite::params![relative_path, absolute_path],
                )
                .map_err(|error| Error::Database(format!("normalize thumbnail path: {error}")))?;
        }
    }
    transaction
        .commit()
        .map_err(|error| Error::Database(format!("commit thumbnail path normalization: {error}")))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn now_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-screenshot-cache-{name}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn insert_thumbnail_record(db_path: &Path, thumb_path: &Path, last_used_at: i64) {
        let conn = Connection::open(db_path).unwrap();
        conn.execute(
            "INSERT INTO screenshot_thumbnail_cache (
                thumb_path, source_path, cache_key, size_bytes, modified_at, created_at, last_used_at
             ) VALUES (?1, 'source.png', 'cache-key', 10, 20, 30, ?2)",
            rusqlite::params![path_string(thumb_path), last_used_at],
        )
        .unwrap();
    }

    fn open_cache(dir: &TestDir) -> MetadataCacheDb {
        MetadataCacheDb::new(&dir.path.join("metadataCache.db")).unwrap()
    }

    fn store_entries(
        cache: &MetadataCacheDb,
        root: &str,
        entries: &[ScreenshotLibraryEntry],
    ) -> Result<()> {
        let seen: HashSet<String> = entries.iter().map(|entry| entry.path.clone()).collect();
        cache.replace_library_entries(root, &seen, entries, false)?;
        Ok(())
    }

    fn library_entry(
        root: &str,
        path: &str,
        folder_path: &str,
        file_name: &str,
    ) -> ScreenshotLibraryEntry {
        ScreenshotLibraryEntry {
            scan_root: root.into(),
            path: path.into(),
            folder_path: folder_path.into(),
            file_name: file_name.into(),
            size_bytes: 100,
            modified_at: 1000,
            created_at: None,
            width: Some(1920),
            height: Some(1080),
            world_id: None,
            world_name: None,
            captured_at: None,
            metadata_json: None,
            error: None,
        }
    }

    #[test]
    fn opening_cache_normalizes_current_absolute_thumbnail_paths() -> Result<()> {
        let dir = TestDir::new("normalize-current");
        let db_path = dir.path.join("metadataCache.db");
        drop(MetadataCacheDb::new(&db_path)?);
        let absolute_path = dir.path.join("ScreenshotThumbs").join("cached.webp");
        insert_thumbnail_record(&db_path, &absolute_path, 42);

        let cache = MetadataCacheDb::new(&db_path)?;
        let entries = cache.thumbnail_cache_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].thumb_path, "cached.webp");
        assert_eq!(entries[0].last_used_at, 42);
        Ok(())
    }

    #[test]
    fn opening_cache_removes_absolute_thumbnail_paths_from_another_directory() -> Result<()> {
        let dir = TestDir::new("normalize-mismatched");
        let db_path = dir.path.join("metadataCache.db");
        drop(MetadataCacheDb::new(&db_path)?);
        let absolute_path = dir.path.join("old-thumbnails").join("cached.webp");
        insert_thumbnail_record(&db_path, &absolute_path, 42);

        let cache = MetadataCacheDb::new(&db_path)?;
        assert!(cache.thumbnail_cache_entries().is_empty());
        Ok(())
    }

    #[test]
    fn folder_tree_for_empty_root_path_is_empty() -> Result<()> {
        let dir = TestDir::new("folder-tree-empty-root");
        let cache = open_cache(&dir);

        let tree = cache.screenshot_folder_tree_for_root("")?;

        assert_eq!(tree.root_path, "");
        assert!(tree.folders.is_empty());
        Ok(())
    }

    #[test]
    fn folder_tree_aggregates_nested_folder_counts_and_keeps_direct_latest_modified() -> Result<()>
    {
        let dir = TestDir::new("folder-tree-nested");
        let cache = open_cache(&dir);
        let root = dir.path.join("Screenshots");
        let subfolder = root.join("2026-01");
        let root_str = path_string(&root);
        let subfolder_str = path_string(&subfolder);

        let mut entry_root_a = library_entry(
            &root_str,
            &path_string(&root.join("root-a.png")),
            &root_str,
            "root-a.png",
        );
        entry_root_a.modified_at = 1000;
        let mut entry_root_b = library_entry(
            &root_str,
            &path_string(&root.join("root-b.png")),
            &root_str,
            "root-b.png",
        );
        entry_root_b.modified_at = 2000;
        let mut entry_sub_a = library_entry(
            &root_str,
            &path_string(&subfolder.join("sub-a.png")),
            &subfolder_str,
            "sub-a.png",
        );
        entry_sub_a.modified_at = 5000;
        store_entries(
            &cache,
            &root_str,
            &[entry_root_a, entry_root_b, entry_sub_a],
        )?;

        let tree = cache.screenshot_folder_tree_for_root(&root_str)?;
        assert_eq!(tree.folders.len(), 2);

        let root_info = tree
            .folders
            .iter()
            .find(|folder| folder.path == root_str)
            .expect("root folder present");
        assert_eq!(root_info.image_count, 2);
        assert_eq!(root_info.total_image_count, 3);
        assert_eq!(root_info.latest_modified_at, Some(2000));
        assert_eq!(root_info.parent_path, None);

        let sub_info = tree
            .folders
            .iter()
            .find(|folder| folder.path == subfolder_str)
            .expect("subfolder present");
        assert_eq!(sub_info.image_count, 1);
        assert_eq!(sub_info.total_image_count, 1);
        assert_eq!(sub_info.latest_modified_at, Some(5000));
        assert_eq!(sub_info.parent_path, Some(root_str.clone()));
        assert_eq!(sub_info.name, "2026-01");
        Ok(())
    }

    #[test]
    fn folder_tree_includes_root_with_zero_own_images_when_only_subfolders_have_files() -> Result<()>
    {
        let dir = TestDir::new("folder-tree-empty-direct");
        let cache = open_cache(&dir);
        let root = dir.path.join("Screenshots");
        let subfolder = root.join("2026-02");
        let root_str = path_string(&root);
        let subfolder_str = path_string(&subfolder);

        let entry = library_entry(
            &root_str,
            &path_string(&subfolder.join("only.png")),
            &subfolder_str,
            "only.png",
        );
        store_entries(&cache, &root_str, &[entry])?;

        let tree = cache.screenshot_folder_tree_for_root(&root_str)?;
        let root_info = tree
            .folders
            .iter()
            .find(|folder| folder.path == root_str)
            .expect("root folder present even without direct images");
        assert_eq!(root_info.image_count, 0);
        assert_eq!(root_info.total_image_count, 1);
        assert_eq!(root_info.latest_modified_at, None);
        Ok(())
    }

    #[test]
    fn replace_library_entries_prunes_files_missing_from_seen_when_enabled() -> Result<()> {
        let dir = TestDir::new("replace-prune-enabled");
        let cache = open_cache(&dir);
        let root_str = path_string(&dir.path.join("Screenshots"));
        let entry_a = library_entry(&root_str, "a.png", &root_str, "a.png");
        let entry_b = library_entry(&root_str, "b.png", &root_str, "b.png");
        let full_seen: HashSet<String> = ["a.png".to_string(), "b.png".to_string()]
            .into_iter()
            .collect();
        cache.replace_library_entries(&root_str, &full_seen, &[entry_a.clone(), entry_b], true)?;
        assert_eq!(cache.library_file_states(&root_str).len(), 2);

        let remaining_seen: HashSet<String> = ["a.png".to_string()].into_iter().collect();
        let deleted =
            cache.replace_library_entries(&root_str, &remaining_seen, &[entry_a], true)?;

        assert_eq!(deleted, 1);
        let states = cache.library_file_states(&root_str);
        assert_eq!(states.len(), 1);
        assert!(states.contains_key("a.png"));
        Ok(())
    }

    #[test]
    fn replace_library_entries_keeps_missing_files_when_pruning_disabled() -> Result<()> {
        let dir = TestDir::new("replace-prune-disabled");
        let cache = open_cache(&dir);
        let root_str = path_string(&dir.path.join("Screenshots"));
        let entry_a = library_entry(&root_str, "a.png", &root_str, "a.png");
        let entry_b = library_entry(&root_str, "b.png", &root_str, "b.png");
        let full_seen: HashSet<String> = ["a.png".to_string(), "b.png".to_string()]
            .into_iter()
            .collect();
        cache.replace_library_entries(&root_str, &full_seen, &[entry_a.clone(), entry_b], true)?;

        let partial_seen: HashSet<String> = ["a.png".to_string()].into_iter().collect();
        let deleted = cache.replace_library_entries(&root_str, &partial_seen, &[entry_a], false)?;

        assert_eq!(deleted, 0);
        assert_eq!(cache.library_file_states(&root_str).len(), 2);
        Ok(())
    }

    #[test]
    fn replace_library_entries_is_idempotent_for_unchanged_entries() -> Result<()> {
        let dir = TestDir::new("replace-idempotent");
        let cache = open_cache(&dir);
        let root_str = path_string(&dir.path.join("Screenshots"));
        let entry = library_entry(&root_str, "a.png", &root_str, "a.png");
        let seen: HashSet<String> = ["a.png".to_string()].into_iter().collect();

        cache.replace_library_entries(&root_str, &seen, std::slice::from_ref(&entry), true)?;
        let deleted = cache.replace_library_entries(&root_str, &seen, &[entry], true)?;

        assert_eq!(deleted, 0);
        let states = cache.library_file_states(&root_str);
        assert_eq!(states.len(), 1);
        let state = states.get("a.png").expect("entry present");
        assert_eq!(state.size_bytes, 100);
        assert_eq!(state.modified_at, 1000);
        assert_eq!(state.index_version, SCREENSHOT_LIBRARY_INDEX_VERSION);
        Ok(())
    }

    #[test]
    fn mark_library_entry_stale_for_test_resets_index_version() -> Result<()> {
        let dir = TestDir::new("mark-stale");
        let cache = open_cache(&dir);
        let root_str = path_string(&dir.path.join("Screenshots"));
        let entry = library_entry(&root_str, "a.png", &root_str, "a.png");
        let seen: HashSet<String> = ["a.png".to_string()].into_iter().collect();
        cache.replace_library_entries(&root_str, &seen, &[entry], true)?;

        cache.mark_library_entry_stale_for_test("a.png")?;

        let states = cache.library_file_states(&root_str);
        let state = states.get("a.png").expect("entry still present");
        assert_eq!(state.index_version, 0);
        Ok(())
    }

    #[test]
    fn list_screenshot_folder_images_for_root_filters_by_folder_and_orders_by_file_name(
    ) -> Result<()> {
        let dir = TestDir::new("list-folder-images");
        let cache = open_cache(&dir);
        let root = dir.path.join("Screenshots");
        let folder_a = root.join("A");
        let folder_b = root.join("B");
        let root_str = path_string(&root);
        let folder_a_str = path_string(&folder_a);
        let folder_b_str = path_string(&folder_b);

        let entries = vec![
            library_entry(
                &root_str,
                &path_string(&folder_a.join("b.png")),
                &folder_a_str,
                "b.png",
            ),
            library_entry(
                &root_str,
                &path_string(&folder_a.join("a.png")),
                &folder_a_str,
                "a.png",
            ),
            library_entry(
                &root_str,
                &path_string(&folder_b.join("c.png")),
                &folder_b_str,
                "c.png",
            ),
        ];
        store_entries(&cache, &root_str, &entries)?;

        let images = cache.list_screenshot_folder_images_for_root(&root_str, &folder_a_str)?;

        assert_eq!(images.len(), 2);
        assert_eq!(images[0].file_name, "a.png");
        assert_eq!(images[1].file_name, "b.png");
        Ok(())
    }

    #[test]
    fn list_world_screenshots_for_root_filters_by_world_id() -> Result<()> {
        let dir = TestDir::new("list-world-screenshots");
        let cache = open_cache(&dir);
        let root_str = path_string(&dir.path.join("Screenshots"));

        let mut entry_world_a = library_entry(&root_str, "a.png", &root_str, "a.png");
        entry_world_a.world_id = Some("wrld_a".into());
        let mut entry_world_b = library_entry(&root_str, "b.png", &root_str, "b.png");
        entry_world_b.world_id = Some("wrld_b".into());
        store_entries(&cache, &root_str, &[entry_world_a, entry_world_b])?;

        let images = cache.list_world_screenshots_for_root(&root_str, "wrld_a")?;

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].path, "a.png");
        assert_eq!(images[0].world_id.as_deref(), Some("wrld_a"));
        Ok(())
    }

    #[test]
    fn record_thumbnail_cache_upserts_existing_entry_by_thumb_path() {
        let dir = TestDir::new("thumbnail-upsert");
        let cache = open_cache(&dir);

        cache.record_thumbnail_cache("source-1.png", "thumb.webp", "key-1", 100, 1000);
        cache.record_thumbnail_cache("source-2.png", "thumb.webp", "key-2", 200, 2000);

        let entries = cache.thumbnail_cache_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_path, "source-2.png");
        assert_eq!(entries[0].cache_key, "key-2");
        assert_eq!(entries[0].size_bytes, 200);
        assert_eq!(entries[0].modified_at, 2000);
    }

    #[test]
    fn thumbnail_cache_entries_for_source_filters_by_source_path() {
        let dir = TestDir::new("thumbnail-by-source");
        let cache = open_cache(&dir);
        cache.record_thumbnail_cache("source-1.png", "thumb-1.webp", "key-1", 100, 1000);
        cache.record_thumbnail_cache("source-1.png", "thumb-1-alt.webp", "key-1", 100, 1000);
        cache.record_thumbnail_cache("source-2.png", "thumb-2.webp", "key-2", 100, 1000);

        let entries = cache.thumbnail_cache_entries_for_source("source-1.png");

        assert_eq!(entries.len(), 2);
        assert!(entries
            .iter()
            .all(|entry| entry.source_path == "source-1.png"));
    }

    #[test]
    fn delete_thumbnail_cache_record_removes_only_matching_entry() {
        let dir = TestDir::new("thumbnail-delete");
        let cache = open_cache(&dir);
        cache.record_thumbnail_cache("source-1.png", "thumb-1.webp", "key-1", 100, 1000);
        cache.record_thumbnail_cache("source-2.png", "thumb-2.webp", "key-2", 100, 1000);

        cache.delete_thumbnail_cache_record("thumb-1.webp");

        let entries = cache.thumbnail_cache_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].thumb_path, "thumb-2.webp");
    }

    #[test]
    fn thumbnail_last_used_map_reflects_recorded_entries() {
        let dir = TestDir::new("thumbnail-last-used-map");
        let cache = open_cache(&dir);
        cache.record_thumbnail_cache("source-1.png", "thumb-1.webp", "key-1", 100, 1000);
        cache.record_thumbnail_cache("source-2.png", "thumb-2.webp", "key-2", 100, 2000);

        let map = cache.thumbnail_last_used_map();

        assert_eq!(map.len(), 2);
        assert!(map.contains_key("thumb-1.webp"));
        assert!(map.contains_key("thumb-2.webp"));
    }

    #[test]
    fn clear_all_removes_metadata_library_and_thumbnail_records() -> Result<()> {
        let dir = TestDir::new("clear-all");
        let cache = open_cache(&dir);
        let root_str = path_string(&dir.path.join("Screenshots"));
        cache.bulk_add(&[("file.png".to_string(), Some("{}".to_string()))]);
        let entry = library_entry(&root_str, "a.png", &root_str, "a.png");
        store_entries(&cache, &root_str, &[entry])?;
        cache.record_thumbnail_cache("a.png", "thumb.webp", "key", 100, 1000);

        cache.clear_all();

        assert!(!cache.is_cached("file.png"));
        assert!(cache.library_file_states(&root_str).is_empty());
        assert!(cache.thumbnail_cache_entries().is_empty());
        Ok(())
    }
}
