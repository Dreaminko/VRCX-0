use std::collections::HashMap;

use crate::wire::{Room, RoomMember};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RoomMemberState {
    pub user_id: String,
    pub display_name: String,
    pub is_self: bool,
    pub is_friend: bool,
    pub joined_at: Option<String>,
    pub languages: Vec<String>,
    pub note: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RoomState {
    pub location: String,
    pub world_id: String,
    pub world_name: String,
    pub destination: String,
    pub entered_at: String,
    pub members: Vec<RoomMemberState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RoomChange {
    Snapshot(RoomState),
    Joined(Vec<RoomMemberState>),
    Left(Vec<String>),
}

pub(crate) fn diff_room(previous: Option<&RoomState>, next: &RoomState) -> Vec<RoomChange> {
    let Some(previous) = previous else {
        return vec![RoomChange::Snapshot(next.clone())];
    };
    if previous.location != next.location
        || previous.world_id != next.world_id
        || previous.world_name != next.world_name
        || previous.destination != next.destination
        || previous.entered_at != next.entered_at
    {
        return vec![RoomChange::Snapshot(next.clone())];
    }

    let previous_by_key = members_by_key(&previous.members);
    let next_by_key = members_by_key(&next.members);
    if (previous_by_key.len() != previous.members.len() || next_by_key.len() != next.members.len())
        && previous.members != next.members
    {
        return vec![RoomChange::Snapshot(next.clone())];
    }
    if next_by_key.iter().any(|(key, member)| {
        previous_by_key
            .get(key)
            .is_some_and(|previous| *previous != *member)
    }) {
        return vec![RoomChange::Snapshot(next.clone())];
    }

    let joined = next
        .members
        .iter()
        .filter(|member| !previous_by_key.contains_key(member.user_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let left = previous
        .members
        .iter()
        .filter(|member| !next_by_key.contains_key(member.user_id.as_str()))
        .map(|member| member.user_id.clone())
        .collect::<Vec<_>>();
    let mut changes = Vec::new();
    if !joined.is_empty() {
        changes.push(RoomChange::Joined(joined));
    }
    if !left.is_empty() {
        changes.push(RoomChange::Left(left));
    }
    changes
}

fn members_by_key(members: &[RoomMemberState]) -> HashMap<&str, &RoomMemberState> {
    members
        .iter()
        .map(|member| (member.user_id.as_str(), member))
        .collect()
}

pub(crate) fn wire_room(room: &RoomState) -> Room {
    Room {
        location: room.location.clone(),
        world_id: room.world_id.clone(),
        world_name: room.world_name.clone(),
        destination: room.destination.clone(),
        entered_at: room.entered_at.clone(),
        members: room.members.iter().map(wire_member).collect(),
    }
}

pub(crate) fn wire_member(member: &RoomMemberState) -> RoomMember {
    RoomMember {
        user_id: member.user_id.clone(),
        display_name: member.display_name.clone(),
        is_self: member.is_self,
        is_friend: member.is_friend,
        joined_at: member.joined_at.clone(),
        languages: member.languages.clone(),
        note: member.note.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(location: &str, members: &[(&str, &str)]) -> RoomState {
        RoomState {
            location: location.into(),
            world_id: location.split(':').next().unwrap_or_default().into(),
            members: members
                .iter()
                .map(|(user_id, display_name)| RoomMemberState {
                    user_id: (*user_id).into(),
                    display_name: (*display_name).into(),
                    ..RoomMemberState::default()
                })
                .collect(),
            ..RoomState::default()
        }
    }

    #[test]
    fn first_room_and_instance_change_emit_snapshots() {
        let first = room("wrld_a:1", &[("usr_a", "Alice")]);
        let second = room("wrld_b:2", &[("usr_a", "Alice")]);
        assert_eq!(
            diff_room(None, &first),
            vec![RoomChange::Snapshot(first.clone())]
        );
        assert_eq!(
            diff_room(Some(&first), &second),
            vec![RoomChange::Snapshot(second.clone())]
        );
    }

    #[test]
    fn same_instance_members_emit_join_and_leave_deltas() {
        let first = room("wrld_a:1", &[("usr_a", "Alice")]);
        let joined = room("wrld_a:1", &[("usr_a", "Alice"), ("usr_b", "Bob")]);
        assert_eq!(
            diff_room(Some(&first), &joined),
            vec![RoomChange::Joined(vec![joined.members[1].clone()])]
        );
        assert_eq!(
            diff_room(Some(&joined), &first),
            vec![RoomChange::Left(vec!["usr_b".into()])]
        );
    }

    #[test]
    fn same_display_name_with_different_ids_stays_distinct() {
        let first = room("wrld_a:1", &[("usr_a", "Twin")]);
        let second = room("wrld_a:1", &[("usr_a", "Twin"), ("usr_b", "Twin")]);
        assert_eq!(
            diff_room(Some(&first), &second),
            vec![RoomChange::Joined(vec![second.members[1].clone()])]
        );
    }

    #[test]
    fn changed_member_metadata_resets_with_snapshot() {
        let first = room("wrld_a:1", &[("usr_a", "Alice")]);
        let mut enriched = first.clone();
        enriched.members[0].languages = vec!["eng".into()];
        assert_eq!(
            diff_room(Some(&first), &enriched),
            vec![RoomChange::Snapshot(enriched.clone())]
        );
    }

    #[test]
    fn blank_user_ids_are_not_replaced_with_display_name_keys() {
        let first = room("wrld_a:1", &[("", "Alice")]);
        let second = room("wrld_a:1", &[("", "Bob")]);
        assert_eq!(
            diff_room(Some(&first), &second),
            vec![RoomChange::Snapshot(second.clone())]
        );
    }

    #[test]
    fn duplicate_blank_user_ids_reset_when_any_member_changes() {
        let first = room("wrld_a:1", &[("", "Alice"), ("", "Bob")]);
        let second = room("wrld_a:1", &[("", "Carol"), ("", "Bob")]);
        assert_eq!(
            diff_room(Some(&first), &second),
            vec![RoomChange::Snapshot(second.clone())]
        );
    }

    #[test]
    fn simultaneous_join_and_leave_emit_both_deltas() {
        let first = room("wrld_a:1", &[("usr_a", "Alice")]);
        let second = room("wrld_a:1", &[("usr_b", "Bob")]);
        assert_eq!(
            diff_room(Some(&first), &second),
            vec![
                RoomChange::Joined(vec![second.members[0].clone()]),
                RoomChange::Left(vec!["usr_a".into()])
            ]
        );
    }
}
