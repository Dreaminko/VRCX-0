use serde::{Deserialize, Serialize};
use specta::Type;

use crate::state::{RoomMemberState, RoomState};

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
#[allow(dead_code)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomMemberRef<'a> {
    user_id: &'a str,
    display_name: &'a str,
    is_self: bool,
    is_friend: bool,
    joined_at: Option<&'a str>,
    languages: &'a [String],
    note: &'a str,
}

impl<'a> From<&'a RoomMemberState> for RoomMemberRef<'a> {
    fn from(member: &'a RoomMemberState) -> Self {
        Self {
            user_id: &member.user_id,
            display_name: &member.display_name,
            is_self: member.is_self,
            is_friend: member.is_friend,
            joined_at: member.joined_at.as_deref(),
            languages: &member.languages,
            note: &member.note,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoomRef<'a> {
    location: &'a str,
    world_id: &'a str,
    world_name: &'a str,
    destination: &'a str,
    entered_at: &'a str,
    members: Vec<RoomMemberRef<'a>>,
}

impl<'a> From<&'a RoomState> for RoomRef<'a> {
    fn from(room: &'a RoomState) -> Self {
        Self {
            location: &room.location,
            world_id: &room.world_id,
            world_name: &room.world_name,
            destination: &room.destination,
            entered_at: &room.entered_at,
            members: room.members.iter().map(RoomMemberRef::from).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub(crate) enum ServerMessageRef<'a> {
    #[serde(rename = "room.snapshot")]
    Snapshot {
        seq: u64,
        at: &'a str,
        room: Option<RoomRef<'a>>,
    },
    #[serde(rename = "room.joined")]
    Joined {
        seq: u64,
        at: &'a str,
        members: Vec<RoomMemberRef<'a>>,
    },
    #[serde(rename = "room.left")]
    Left {
        seq: u64,
        at: &'a str,
        #[serde(rename = "userIds")]
        user_ids: &'a [String],
    },
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
