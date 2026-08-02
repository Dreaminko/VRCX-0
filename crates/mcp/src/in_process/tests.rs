use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Map};
use vrcx_0_application_core::{
    RuntimeTask, RuntimeTaskExecutor, RuntimeTaskHandle, TaskSupervisor,
};
use vrcx_0_core::friends::FriendRecord;

use super::*;
use crate::test_support::test_runtime;

#[derive(Clone, Copy)]
struct DiscardTaskExecutor;

struct FinishedTaskHandle;

impl RuntimeTaskExecutor for DiscardTaskExecutor {
    fn spawn(&self, _task: RuntimeTask) -> Box<dyn RuntimeTaskHandle> {
        Box::new(FinishedTaskHandle)
    }
}

impl RuntimeTaskHandle for FinishedTaskHandle {
    fn abort(&self) {}

    fn is_finished(&self) -> bool {
        true
    }

    fn join_or_abort(&mut self, _timeout: Duration) {}
}

fn friend(id: &str, display_name: &str, state_bucket: &str, location: &str) -> FriendRecord {
    FriendRecord {
        id: id.into(),
        display_name: display_name.into(),
        state: state_bucket.into(),
        state_bucket: state_bucket.into(),
        location: location.into(),
        status: "active".into(),
        ..FriendRecord::default()
    }
}

fn seed_live_friends(runtime: &mut McpRuntime, friends: HashMap<String, FriendRecord>) {
    runtime.tasks.set_executor(DiscardTaskExecutor);
    runtime
        .realtime_runtime
        .start(
            "usr_owner".into(),
            "https://api.vrchat.cloud/api/1".into(),
            "wss://pipeline.vrchat.cloud".into(),
            1,
            json!({ "id": "usr_owner" }),
            friends,
        )
        .unwrap();
    runtime.tasks = TaskSupervisor::new();
}

#[tokio::test]
async fn in_process_bridge_lists_the_real_server_tools() {
    let (_dir, runtime) = test_runtime("in-process-list", "usr_owner").unwrap();
    let tools = spawn_in_process_tools(runtime).await.unwrap();

    let descriptors = tools.list_tools().await.unwrap();
    let favorites = descriptors
        .iter()
        .find(|tool| tool.name == "get_favorites")
        .expect("get_favorites should cross the in-process bridge");

    assert!(!favorites.description.is_empty());
    assert_eq!(favorites.parameters["type"], "object");
}

#[tokio::test]
async fn in_process_bridge_returns_structured_read_only_results() {
    let (_dir, runtime) = test_runtime("in-process-call", "usr_owner").unwrap();
    let tools = spawn_in_process_tools(runtime).await.unwrap();

    let outcome = tools
        .call_tool("get_favorites", Some(Map::new()))
        .await
        .unwrap();

    assert!(!outcome.is_error);
    let structured = outcome.structured.expect("structured tool result");
    assert_eq!(structured["rows"], json!([]));
    assert!(structured["summary"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

#[tokio::test]
async fn in_process_bridge_preserves_tool_errors_and_dispatch_failures() {
    let (_dir, runtime) = test_runtime("in-process-errors", "usr_owner").unwrap();
    let tools = spawn_in_process_tools(runtime).await.unwrap();

    let invalid = tools
        .call_tool(
            "get_favorites",
            Some(Map::from_iter([("kind".into(), json!(42))])),
        )
        .await
        .unwrap();
    assert!(invalid.is_error);
    assert!(!invalid.text.is_empty());

    let unknown = tools.call_tool("missing_tool", Some(Map::new())).await;
    assert!(unknown
        .unwrap_err()
        .to_string()
        .contains("call_tool failed"));
}

#[tokio::test]
async fn online_friends_tool_filters_sorts_and_projects_live_presence() {
    let (_dir, mut runtime) = test_runtime("in-process-online-friends", "usr_owner").unwrap();
    let mut friends = HashMap::new();
    let mut alpha = friend(
        "usr_alpha",
        "Alpha",
        "active",
        "wrld_alpha:123~group(grp_alpha)",
    );
    alpha.last_platform = "android".into();
    alpha
        .extra
        .insert("world_name".into(), json!("Alpha World"));
    let mut zulu = friend("usr_zulu", "Zulu", "online", "wrld_zulu:456");
    zulu.platform = "standalonewindows".into();
    zulu.extra.insert("worldName".into(), json!("Zulu World"));
    friends.insert(alpha.id.clone(), alpha);
    friends.insert(zulu.id.clone(), zulu);
    friends.insert(
        "usr_offline".into(),
        friend("usr_offline", "Offline", "offline", "offline"),
    );
    seed_live_friends(&mut runtime, friends);
    let tools = spawn_in_process_tools(runtime).await.unwrap();

    let outcome = tools
        .call_tool("get_online_friends", Some(Map::new()))
        .await
        .unwrap();
    let structured = outcome.structured.expect("structured tool result");

    assert_eq!(structured["summary"], "2 friends online now: Alpha, Zulu.");
    assert_eq!(structured["rows"][0]["userId"], "usr_alpha");
    assert_eq!(structured["rows"][0]["worldId"], "wrld_alpha");
    assert_eq!(structured["rows"][0]["worldName"], "Alpha World");
    assert_eq!(structured["rows"][0]["instanceAccessType"], "group");
    assert_eq!(structured["rows"][0]["platform"], "android");
    assert_eq!(structured["rows"][1]["userId"], "usr_zulu");
    assert_eq!(structured["rows"][1]["platform"], "standalonewindows");
}

#[tokio::test]
async fn online_friends_tool_honors_custom_states_and_location_redaction() {
    let (_dir, mut runtime) = test_runtime("in-process-offline-friends", "usr_owner").unwrap();
    let offline = friend("usr_offline", "Offline", "offline", "offline");
    seed_live_friends(
        &mut runtime,
        [(offline.id.clone(), offline)].into_iter().collect(),
    );
    let tools = spawn_in_process_tools(runtime).await.unwrap();

    let outcome = tools
        .call_tool(
            "get_online_friends",
            Some(Map::from_iter([
                ("states".into(), json!([" OFFLINE ", ""])),
                ("includeLocation".into(), json!(false)),
            ])),
        )
        .await
        .unwrap();
    let structured = outcome.structured.expect("structured tool result");

    assert_eq!(structured["rows"].as_array().unwrap().len(), 1);
    assert_eq!(structured["rows"][0]["userId"], "usr_offline");
    assert!(structured["rows"][0]["location"].is_null());
    assert!(structured["rows"][0]["worldId"].is_null());
    assert!(structured["rows"][0]["worldName"].is_null());
    assert!(structured["rows"][0]["instanceAccessType"].is_null());
}
