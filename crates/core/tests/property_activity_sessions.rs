mod support;

use proptest::prelude::*;
use support::{presence_events, valid_sessions};
use vrcx_0_core::activity_sessions::{
    merge_sessions_with_gap, sessions_from_presence, ActivitySession, PresenceKind,
};

fn interval_projection(sessions: &[ActivitySession]) -> Vec<(i64, i64, bool)> {
    sessions
        .iter()
        .map(|session| (session.start, session.end, session.is_open_tail))
        .collect()
}

proptest! {
    #[test]
    fn merged_sessions_are_sorted_and_fully_coalesced(
        older in valid_sessions(24),
        newer in valid_sessions(24),
        merge_gap_ms in 0i64..=10 * 60_000,
    ) {
        let merged = merge_sessions_with_gap(&older, &newer, merge_gap_ms);

        for pair in merged.windows(2) {
            prop_assert!(pair[0].start <= pair[1].start);
            prop_assert!(pair[1].start > pair[0].end + merge_gap_ms);
        }
    }

    #[test]
    fn merged_sessions_cover_every_input_interval(
        older in valid_sessions(24),
        newer in valid_sessions(24),
        merge_gap_ms in 0i64..=10 * 60_000,
    ) {
        let merged = merge_sessions_with_gap(&older, &newer, merge_gap_ms);

        for input in older.iter().chain(&newer) {
            let is_covered = merged.iter().any(|output| {
                output.start <= input.start && output.end >= input.end
            });
            prop_assert!(is_covered);
        }
    }

    #[test]
    fn merging_is_idempotent(
        older in valid_sessions(24),
        newer in valid_sessions(24),
        merge_gap_ms in 0i64..=10 * 60_000,
    ) {
        let merged = merge_sessions_with_gap(&older, &newer, merge_gap_ms);
        let merged_again = merge_sessions_with_gap(&merged, &[], merge_gap_ms);

        prop_assert_eq!(merged_again, merged);
    }

    #[test]
    fn interval_merge_is_independent_of_input_partition(
        sessions in valid_sessions(48),
        split_at in 0usize..=48,
        merge_gap_ms in 0i64..=10 * 60_000,
    ) {
        let split_at = split_at.min(sessions.len());
        let from_partition =
            merge_sessions_with_gap(&sessions[..split_at], &sessions[split_at..], merge_gap_ms);
        let from_single_input = merge_sessions_with_gap(&sessions, &[], merge_gap_ms);

        prop_assert_eq!(
            interval_projection(&from_partition),
            interval_projection(&from_single_input)
        );
    }

    #[test]
    fn sorted_presence_events_only_produce_valid_intervals(
        events in presence_events(),
        initial_start in prop::option::of(0i64..=support::BASE_MS),
    ) {
        let (_, sessions) = sessions_from_presence(&events, initial_start);

        prop_assert!(sessions.iter().all(|session| session.start <= session.end));
    }

    #[test]
    fn other_presence_events_do_not_change_state(
        events in presence_events(),
        initial_start in prop::option::of(0i64..=support::BASE_MS),
    ) {
        let without_other: Vec<_> = events
            .iter()
            .copied()
            .filter(|(_, kind)| *kind != PresenceKind::Other)
            .collect();

        prop_assert_eq!(
            sessions_from_presence(&events, initial_start),
            sessions_from_presence(&without_other, initial_start)
        );
    }
}
