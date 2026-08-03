use vrcx_0_persistence::realtime::{
    AvatarHistoryUpsert, AvatarTimeSpentUpsert, RealtimePersistenceBatch,
};

use crate::realtime::RealtimeCurrentUserAuthority;

use super::state::RealtimeCurrentUserStateSnapshot;
use super::utils::{first_positive, EventTime};

pub(super) fn apply_avatar_wear_transition(
    mut next: RealtimeCurrentUserStateSnapshot,
    previous: &RealtimeCurrentUserStateSnapshot,
    authority: &RealtimeCurrentUserAuthority,
    now: &EventTime,
    records_current_avatar_history: bool,
) -> (RealtimeCurrentUserStateSnapshot, RealtimePersistenceBatch) {
    let previous_avatar_id = previous.current_avatar.clone();
    let next_avatar_id = next.current_avatar.clone();
    let previous_swap_time = previous.previous_avatar_swap_time;
    let mut persistence = RealtimePersistenceBatch::default();

    if !authority.is_available() {
        next.previous_avatar_swap_time = previous_swap_time;
        match previous.raw.get("$previousAvatarSwapTime").cloned() {
            Some(value) => {
                next.raw.insert("$previousAvatarSwapTime".into(), value);
            }
            None => {
                next.raw.remove("$previousAvatarSwapTime");
            }
        }
        return (next, persistence);
    }

    if !authority.is_game_running() {
        if !previous_avatar_id.is_empty() && previous_swap_time > 0 {
            persistence
                .avatar_time_spent_upserts
                .push(AvatarTimeSpentUpsert {
                    avatar_id: previous_avatar_id,
                    created_at: now.iso.clone(),
                    time_spent: now.timestamp_ms.saturating_sub(previous_swap_time),
                });
        }
        next.set_previous_avatar_swap_time(None);
        return (next, persistence);
    }
    if next_avatar_id.is_empty() {
        next.set_previous_avatar_swap_time((previous_swap_time > 0).then_some(previous_swap_time));
        return (next, persistence);
    }
    if previous_avatar_id.is_empty() {
        let swap_time = first_positive([next.previous_avatar_swap_time, now.timestamp_ms]);
        next.set_previous_avatar_swap_time(Some(swap_time));
        persistence
            .avatar_history_upserts
            .push(AvatarHistoryUpsert {
                avatar_id: next_avatar_id,
                created_at: now.iso.clone(),
            });
        return (next, persistence);
    }
    if previous_avatar_id != next_avatar_id {
        next.set_previous_avatar_swap_time(Some(now.timestamp_ms));
        persistence
            .avatar_history_upserts
            .push(AvatarHistoryUpsert {
                avatar_id: next_avatar_id,
                created_at: now.iso.clone(),
            });
        if previous_swap_time > 0 {
            persistence
                .avatar_time_spent_upserts
                .push(AvatarTimeSpentUpsert {
                    avatar_id: previous_avatar_id,
                    created_at: now.iso.clone(),
                    time_spent: now.timestamp_ms.saturating_sub(previous_swap_time),
                });
        }
        return (next, persistence);
    }
    let next_swap_time = next.previous_avatar_swap_time;
    if records_current_avatar_history || (previous_swap_time <= 0 && next_swap_time <= 0) {
        persistence
            .avatar_history_upserts
            .push(AvatarHistoryUpsert {
                avatar_id: next_avatar_id,
                created_at: now.iso.clone(),
            });
    }
    next.set_previous_avatar_swap_time(Some(first_positive([
        previous_swap_time,
        next_swap_time,
        now.timestamp_ms,
    ])));
    (next, persistence)
}
