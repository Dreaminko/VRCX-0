use std::path::PathBuf;

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
            "vrcx-0-mutual-graph-{name}-{}-{nonce}",
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

fn entry(friend_id: &str, mutual_ids: &[&str]) -> MutualGraphSnapshotEntryInput {
    MutualGraphSnapshotEntryInput {
        friend_id: friend_id.into(),
        mutual_ids: mutual_ids.iter().map(|id| (*id).into()).collect(),
    }
}

fn meta(friend_id: &str, opted_out: bool) -> MutualGraphMetaInput {
    MutualGraphMetaInput {
        friend_id: friend_id.into(),
        last_fetched_at: "2026-07-21T12:00:00Z".into(),
        opted_out,
    }
}

#[test]
fn full_snapshot_commit_removes_opted_out_nodes_that_are_no_longer_friends() {
    let dir = TestDir::new("remove-stale-opt-out");
    let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap();
    let user_id = "usr_self".to_string();

    mutual_graph_snapshot_commit(
        &db,
        user_id.clone(),
        vec![entry("usr_old", &["usr_mutual_old"])],
        vec![meta("usr_old", false)],
    )
    .unwrap();
    mutual_graph_snapshot_commit(
        &db,
        user_id.clone(),
        Vec::new(),
        vec![meta("usr_old", true)],
    )
    .unwrap();
    mutual_graph_snapshot_commit(
        &db,
        user_id.clone(),
        vec![entry("usr_current", &["usr_mutual_current"])],
        vec![meta("usr_current", false)],
    )
    .unwrap();

    let snapshot = mutual_graph_snapshot_get(&db, user_id).unwrap();
    assert_eq!(snapshot.friend_ids, vec!["usr_current"]);
    assert_eq!(
        snapshot
            .links
            .iter()
            .map(|link| (link.friend_id.as_str(), link.mutual_id.as_str()))
            .collect::<Vec<_>>(),
        vec![("usr_current", "usr_mutual_current")]
    );
    assert_eq!(
        snapshot
            .meta
            .iter()
            .map(|entry| entry.friend_id.as_str())
            .collect::<Vec<_>>(),
        vec!["usr_current"]
    );
}
