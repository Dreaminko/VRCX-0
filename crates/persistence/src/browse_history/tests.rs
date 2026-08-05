use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

struct TestDatabase {
    _dir: TestDir,
    db: DatabaseService,
}

struct TestDir {
    path: std::path::PathBuf,
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn test_db(name: &str) -> TestDatabase {
    let path = std::env::temp_dir().join(format!(
        "vrcx-0-browse-history-{name}-{}-{}",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    let db = DatabaseService::new(&path.join("VRCX-0.sqlite3")).unwrap();
    TestDatabase {
        _dir: TestDir { path },
        db,
    }
}

fn record_input(entity_id: &str) -> BrowseHistoryRecordInput {
    BrowseHistoryRecordInput {
        owner_user_id: "usr_owner".into(),
        entity_kind: BrowseHistoryEntityKind::World,
        entity_id: entity_id.into(),
        title: "World".into(),
        image_url: "https://example.com/world.png".into(),
        record_visit: true,
    }
}

fn query(
    db: &DatabaseService,
    cursor: Option<BrowseHistoryCursor>,
    limit: i64,
) -> BrowseHistoryPageOutput {
    browse_history_query(
        db,
        BrowseHistoryQueryInput {
            owner_user_id: "usr_owner".into(),
            entity_kind: None,
            search: String::new(),
            cursor,
            limit,
            date_from: String::new(),
            date_to: String::new(),
        },
    )
    .unwrap()
}

#[test]
fn repeated_visits_update_one_entity_row() {
    let test = test_db("repeat");
    browse_history_record(&test.db, record_input("wrld_repeat")).unwrap();
    browse_history_record(&test.db, record_input("wrld_repeat")).unwrap();

    let page = query(&test.db, None, 10);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].view_count, 2);
    assert_eq!(page.items[0].entity_id, "wrld_repeat");
}

#[test]
fn enrichment_updates_snapshot_without_counting_a_visit() {
    let test = test_db("enrich");
    let mut input = record_input("wrld_enrich");
    input.title.clear();
    input.image_url.clear();
    browse_history_record(&test.db, input).unwrap();
    browse_history_record(
        &test.db,
        BrowseHistoryRecordInput {
            owner_user_id: "usr_owner".into(),
            entity_kind: BrowseHistoryEntityKind::World,
            entity_id: "wrld_enrich".into(),
            title: "Resolved World".into(),
            image_url: "https://example.com/resolved.png".into(),
            record_visit: false,
        },
    )
    .unwrap();

    let page = query(&test.db, None, 10);
    assert_eq!(page.items[0].view_count, 1);
    assert_eq!(page.items[0].title, "Resolved World");
}

#[test]
fn cursor_pagination_is_stable_for_equal_timestamps() {
    let test = test_db("cursor");
    ensure_browse_history_table(&test.db).unwrap();
    for entity_id in ["wrld_a", "wrld_b", "wrld_c"] {
        test.db
            .execute_non_query(
                "INSERT INTO browse_history (
                    owner_user_id, entity_kind, entity_id, title, image_url,
                    first_viewed_at, last_viewed_at, view_count
                 ) VALUES (
                    'usr_owner', 'world', @entity_id, @entity_id, '',
                    '2099-01-01T00:00:00.000Z', '2099-01-01T00:00:00.000Z', 1
                 )",
                &ParamsBuilder::new().set("entity_id", entity_id).build(),
            )
            .unwrap();
    }

    let first = query(&test.db, None, 2);
    assert_eq!(
        first
            .items
            .iter()
            .map(|item| item.entity_id.as_str())
            .collect::<Vec<_>>(),
        vec!["wrld_a", "wrld_b"]
    );
    let second = query(&test.db, first.next_cursor, 2);
    assert_eq!(second.items.len(), 1);
    assert_eq!(second.items[0].entity_id, "wrld_c");
    assert!(second.next_cursor.is_none());
}

#[test]
fn queries_and_clear_are_scoped_to_owner() {
    let test = test_db("owner");
    browse_history_record(&test.db, record_input("wrld_owner_a")).unwrap();
    let mut other = record_input("wrld_owner_b");
    other.owner_user_id = "usr_other".into();
    browse_history_record(&test.db, other).unwrap();

    browse_history_clear(&test.db, "usr_owner".into(), None).unwrap();
    assert!(query(&test.db, None, 10).items.is_empty());
    let other_page = browse_history_query(
        &test.db,
        BrowseHistoryQueryInput {
            owner_user_id: "usr_other".into(),
            entity_kind: None,
            search: String::new(),
            cursor: None,
            limit: 10,
            date_from: String::new(),
            date_to: String::new(),
        },
    )
    .unwrap();
    assert_eq!(other_page.items.len(), 1);
}

#[test]
fn own_profile_is_never_recorded() {
    let test = test_db("self");
    browse_history_record(
        &test.db,
        BrowseHistoryRecordInput {
            owner_user_id: "usr_owner".into(),
            entity_kind: BrowseHistoryEntityKind::User,
            entity_id: "usr_owner".into(),
            title: "Me".into(),
            image_url: String::new(),
            record_visit: true,
        },
    )
    .unwrap();
    browse_history_record(&test.db, record_input("wrld_other")).unwrap();

    let entity_ids = query(&test.db, None, 10)
        .items
        .into_iter()
        .map(|item| item.entity_id)
        .collect::<Vec<_>>();
    assert_eq!(entity_ids, vec!["wrld_other"]);
}

#[test]
fn date_range_filters_by_last_viewed_at() {
    let test = test_db("date-range");
    ensure_browse_history_table(&test.db).unwrap();
    for (entity_id, last_viewed_at) in [
        ("wrld_old", "2099-01-01T10:00:00.000Z"),
        ("wrld_new", "2099-01-03T10:00:00.000Z"),
    ] {
        test.db
            .execute_non_query(
                "INSERT INTO browse_history (
                    owner_user_id, entity_kind, entity_id, title, image_url,
                    first_viewed_at, last_viewed_at, view_count
                 ) VALUES (
                    'usr_owner', 'world', @entity_id, @entity_id, '',
                    @last_viewed_at, @last_viewed_at, 1
                 )",
                &ParamsBuilder::new()
                    .set("entity_id", entity_id)
                    .set("last_viewed_at", last_viewed_at)
                    .build(),
            )
            .unwrap();
    }

    let page = browse_history_query(
        &test.db,
        BrowseHistoryQueryInput {
            owner_user_id: "usr_owner".into(),
            entity_kind: None,
            search: String::new(),
            cursor: None,
            limit: 10,
            date_from: "2099-01-02T00:00:00.000Z".into(),
            date_to: "2099-01-04T00:00:00.000Z".into(),
        },
    )
    .unwrap();

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].entity_id, "wrld_new");
}

#[test]
fn retention_setting_rejects_unsupported_values() {
    let test = test_db("retention");
    assert!(browse_history_retention_days_set(&test.db, 14).is_err());
    assert_eq!(browse_history_retention_days_set(&test.db, 90).unwrap(), 90);
    assert_eq!(browse_history_retention_days_get(&test.db).unwrap(), 90);
}

#[test]
fn shorter_retention_prunes_expired_rows_for_every_account() {
    let test = test_db("retention-prune");
    ensure_browse_history_table(&test.db).unwrap();
    for owner_user_id in ["usr_owner", "usr_other"] {
        test.db
            .execute_non_query(
                "INSERT INTO browse_history (
                    owner_user_id, entity_kind, entity_id, title, image_url,
                    first_viewed_at, last_viewed_at, view_count
                 ) VALUES (
                    @owner_user_id, 'world', @owner_user_id, '', '',
                    '2000-01-01T00:00:00.000Z', '2000-01-01T00:00:00.000Z', 1
                 )",
                &ParamsBuilder::new()
                    .set("owner_user_id", owner_user_id)
                    .build(),
            )
            .unwrap();
    }

    browse_history_retention_days_set(&test.db, 7).unwrap();

    assert!(test
        .db
        .execute("SELECT entity_id FROM browse_history", &Default::default())
        .unwrap()
        .is_empty());
}
