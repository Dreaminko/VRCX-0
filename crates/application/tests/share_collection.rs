use std::path::PathBuf;

use serde_json::json;
use vrcx_0_application::{
    get_or_create_share_owner_key, prepare_share_collection_payload, share_collection_owner_hint,
    ShareCollectionCreateInput, ShareCollectionDeps, SHARE_COLLECTION_MAX_WORLDS,
};
use vrcx_0_persistence::{
    cache_entities::CacheEntityInput, memos::memo_save_world, worlds::world_cache_upsert,
    DatabaseService,
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
        let path = std::env::temp_dir().join(format!(
            "vrcx0-share-collection-{name}-{}-{nonce}",
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

fn test_services(name: &str) -> (TestDir, DatabaseService) {
    let dir = TestDir::new(name);
    let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap();
    (dir, db)
}

fn world_entry(id: &str, release_status: &str, name: &str) -> CacheEntityInput {
    CacheEntityInput {
        id: json!(id),
        author_id: json!("usr_author"),
        author_name: json!("World Author"),
        created_at: json!("2026-01-01T00:00:00.000Z"),
        description: json!("Description"),
        image_url: json!(format!("https://images.example/{id}.png")),
        name: json!(name),
        release_status: json!(release_status),
        thumbnail_image_url: json!(""),
        updated_at: json!("2026-01-02T00:00:00.000Z"),
        version: json!(1),
    }
}

fn is_base64_url_no_pad(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn is_lowercase_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[test]
fn owner_key_is_stable_for_same_user_and_isolated_across_users() {
    let (_dir, db) = test_services("owner-key");

    let owner_key = get_or_create_share_owner_key(&db, " usr_current ").unwrap();
    let same_owner_key = get_or_create_share_owner_key(&db, "usr_current").unwrap();
    let other_owner_key = get_or_create_share_owner_key(&db, "usr_other").unwrap();

    assert_eq!(owner_key, same_owner_key);
    assert_ne!(owner_key, other_owner_key);
    assert_ne!(owner_key, "usr_current");
    assert!(is_base64_url_no_pad(&owner_key));
    assert!(is_base64_url_no_pad(&other_owner_key));
}

#[test]
fn owner_key_requires_authenticated_user() {
    let (_dir, db) = test_services("owner-key-empty");

    let error = get_or_create_share_owner_key(&db, "  ").unwrap_err();
    assert!(error
        .to_string()
        .contains("Share collection requires an authenticated user"));
}

#[test]
fn owner_hint_is_deterministic_and_isolated_across_users() {
    let owner_hint = share_collection_owner_hint(" usr_current ");
    let same_owner_hint = share_collection_owner_hint("usr_current");
    let other_owner_hint = share_collection_owner_hint("usr_other");

    assert_eq!(owner_hint, same_owner_hint);
    assert_ne!(owner_hint, other_owner_hint);
    assert!(is_lowercase_hex_sha256(&owner_hint));
    assert!(is_lowercase_hex_sha256(&other_owner_hint));
}

#[test]
fn prepare_payload_gets_or_creates_owner_key_and_keeps_only_public_worlds_in_input_order() {
    let (_dir, db) = test_services("payload");
    world_cache_upsert(
        &db,
        world_entry(
            "wrld_11111111-1111-1111-1111-111111111111",
            "public",
            "First",
        ),
    )
    .unwrap();
    world_cache_upsert(
        &db,
        world_entry(
            "wrld_22222222-2222-2222-2222-222222222222",
            "private",
            "Private",
        ),
    )
    .unwrap();
    world_cache_upsert(
        &db,
        world_entry(
            "wrld_33333333-3333-3333-3333-333333333333",
            "public",
            "Second",
        ),
    )
    .unwrap();
    memo_save_world(
        &db,
        "wrld_33333333-3333-3333-3333-333333333333".to_string(),
        "Bring friends".to_string(),
    )
    .unwrap();

    let prepared = prepare_share_collection_payload(
        ShareCollectionDeps {
            db: &db,
            current_user_id: "usr_current",
            current_user_display_name: " Scenic Curator ",
        },
        ShareCollectionCreateInput {
            title: " Scenic picks ".to_string(),
            listed: true,
            include_notes: true,
            world_ids: vec![
                "wrld_33333333-3333-3333-3333-333333333333".to_string(),
                "not-world".to_string(),
                "wrld_22222222-2222-2222-2222-222222222222".to_string(),
                "wrld_11111111-1111-1111-1111-111111111111".to_string(),
                "wrld_33333333-3333-3333-3333-333333333333".to_string(),
            ],
        },
    )
    .unwrap();

    assert_eq!(prepared.payload.schema, 1);
    assert_eq!(
        prepared.payload.owner_key,
        get_or_create_share_owner_key(&db, "usr_current").unwrap()
    );
    assert_eq!(
        prepared.payload.owner_hint,
        share_collection_owner_hint("usr_current")
    );
    assert_eq!(prepared.payload.title, "Scenic picks");
    assert!(prepared.payload.listed);
    assert_eq!(prepared.payload.access, "open");
    assert_eq!(prepared.payload.author_name, "Scenic Curator");
    assert!(prepared.payload.updated_at > 0);
    assert_eq!(prepared.payload.worlds.len(), 2);
    assert_eq!(
        prepared.payload.worlds[0].world_id,
        "wrld_33333333-3333-3333-3333-333333333333"
    );
    assert_eq!(prepared.payload.worlds[0].name, "Second");
    assert_eq!(prepared.payload.worlds[0].author_id, "usr_author");
    assert_eq!(
        prepared.payload.worlds[0].created_at,
        "2026-01-01T00:00:00.000Z"
    );
    assert_eq!(prepared.payload.worlds[0].release_status, "public");
    assert_eq!(prepared.payload.worlds[0].version, 1);
    assert_eq!(prepared.payload.worlds[0].comment, "Bring friends");
    assert_eq!(
        prepared.payload.worlds[1].world_id,
        "wrld_11111111-1111-1111-1111-111111111111"
    );
    assert_eq!(prepared.payload.worlds[1].comment, "");
}

#[test]
fn prepare_payload_requires_current_user_id_for_owner_key() {
    let (_dir, db) = test_services("owner-key-payload");
    world_cache_upsert(
        &db,
        world_entry(
            "wrld_11111111-1111-1111-1111-111111111111",
            "public",
            "First",
        ),
    )
    .unwrap();

    let prepared = prepare_share_collection_payload(
        ShareCollectionDeps {
            db: &db,
            current_user_id: " ",
            current_user_display_name: "",
        },
        ShareCollectionCreateInput {
            title: "Worlds".to_string(),
            listed: false,
            include_notes: false,
            world_ids: vec!["wrld_11111111-1111-1111-1111-111111111111".to_string()],
        },
    );

    let error = prepared.unwrap_err();
    assert!(error
        .to_string()
        .contains("Share collection requires an authenticated user"));
}

#[test]
fn prepare_payload_limits_large_groups_to_the_share_cap() {
    let (_dir, db) = test_services("cap");
    let world_ids = (0..(SHARE_COLLECTION_MAX_WORLDS + 3))
        .map(|index| format!("wrld_{index:08x}-1111-1111-1111-111111111111"))
        .collect::<Vec<_>>();
    for world_id in &world_ids {
        world_cache_upsert(&db, world_entry(world_id, "public", world_id)).unwrap();
    }

    let prepared = prepare_share_collection_payload(
        ShareCollectionDeps {
            db: &db,
            current_user_id: "usr_current",
            current_user_display_name: "Current User",
        },
        ShareCollectionCreateInput {
            title: "Large group".to_string(),
            listed: false,
            include_notes: false,
            world_ids: world_ids.clone(),
        },
    )
    .unwrap();

    assert_eq!(prepared.payload.worlds.len(), SHARE_COLLECTION_MAX_WORLDS);
    assert_eq!(prepared.payload.worlds[0].world_id, world_ids[0]);
    assert_eq!(
        prepared.payload.worlds[SHARE_COLLECTION_MAX_WORLDS - 1].world_id,
        world_ids[SHARE_COLLECTION_MAX_WORLDS - 1]
    );
}
