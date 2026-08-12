use serde::{Deserialize, Serialize};
use specta::Type;

pub const PROTOCOL_VERSION: u32 = 1;
pub(crate) const HEARTBEAT_SECONDS: u64 = 20;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RoomMember {
    pub user_id: String,
    pub display_name: String,
    pub is_self: bool,
    pub is_friend: bool,
    pub joined_at: Option<String>,
    pub languages: Vec<String>,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub location: String,
    pub world_id: String,
    pub world_name: String,
    pub destination: String,
    pub entered_at: String,
    pub members: Vec<RoomMember>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ByeReason {
    GameStopped,
    Disabled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Type)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "hello")]
    Hello {
        seq: u64,
        protocol: u32,
        app: String,
        #[serde(rename = "appVersion")]
        app_version: String,
        scopes: Vec<String>,
        #[serde(rename = "heartbeatSec")]
        heartbeat_sec: u64,
    },
    #[serde(rename = "room.snapshot")]
    RoomSnapshot {
        seq: u64,
        at: String,
        room: Option<Room>,
    },
    #[serde(rename = "room.joined")]
    RoomJoined {
        seq: u64,
        at: String,
        members: Vec<RoomMember>,
    },
    #[serde(rename = "room.left")]
    RoomLeft {
        seq: u64,
        at: String,
        #[serde(rename = "userIds")]
        user_ids: Vec<String>,
    },
    #[serde(rename = "ping")]
    Ping { seq: u64, at: String },
    #[serde(rename = "bye")]
    Bye { reason: ByeReason },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "resync")]
    Resync,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn snapshot_fields_and_null_room_are_stable() {
        let value = serde_json::to_value(ServerMessage::RoomSnapshot {
            seq: 1,
            at: "2026-08-12T09:31:00.000Z".into(),
            room: None,
        })
        .unwrap();

        assert_eq!(
            value,
            json!({
                "type": "room.snapshot",
                "seq": 1,
                "at": "2026-08-12T09:31:00.000Z",
                "room": null
            })
        );
    }

    #[test]
    fn hello_fields_are_stable() {
        assert_eq!(
            serde_json::to_value(ServerMessage::Hello {
                seq: 0,
                protocol: 1,
                app: "vrcx-0".into(),
                app_version: "1.2.3".into(),
                scopes: vec!["room".into()],
                heartbeat_sec: 20,
            })
            .unwrap(),
            json!({
                "type": "hello",
                "seq": 0,
                "protocol": 1,
                "app": "vrcx-0",
                "appVersion": "1.2.3",
                "scopes": ["room"],
                "heartbeatSec": 20
            })
        );
    }

    #[test]
    fn member_fields_are_never_omitted() {
        let member = RoomMember {
            user_id: String::new(),
            display_name: String::new(),
            is_self: false,
            is_friend: false,
            joined_at: None,
            languages: Vec::new(),
            note: String::new(),
        };

        assert_eq!(
            serde_json::to_value(member).unwrap(),
            json!({
                "userId": "",
                "displayName": "",
                "isSelf": false,
                "isFriend": false,
                "joinedAt": null,
                "languages": [],
                "note": ""
            })
        );
    }

    #[test]
    fn bye_reasons_keep_protocol_spelling() {
        assert_eq!(
            serde_json::to_value(ServerMessage::Bye {
                reason: ByeReason::GameStopped
            })
            .unwrap(),
            json!({ "type": "bye", "reason": "gameStopped" })
        );
        assert_eq!(
            serde_json::to_value(ServerMessage::Bye {
                reason: ByeReason::Disabled
            })
            .unwrap(),
            json!({ "type": "bye", "reason": "disabled" })
        );
    }

    #[test]
    fn delta_and_ping_field_names_are_stable() {
        assert_eq!(
            serde_json::to_value(ServerMessage::RoomLeft {
                seq: 2,
                at: "now".into(),
                user_ids: vec!["usr_a".into()],
            })
            .unwrap(),
            json!({
                "type": "room.left",
                "seq": 2,
                "at": "now",
                "userIds": ["usr_a"]
            })
        );
        assert_eq!(
            serde_json::to_value(ServerMessage::Ping {
                seq: 3,
                at: "now".into(),
            })
            .unwrap(),
            json!({ "type": "ping", "seq": 3, "at": "now" })
        );
    }
}
