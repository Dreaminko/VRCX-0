mod support;

use std::collections::BTreeSet;

use proptest::prelude::*;
use support::{minute_aligned_sessions, BASE_MS, MINUTE_MS};
use vrcx_0_core::activity_buckets::{
    activity_streaks, activity_timeline, ActivityBucket, ActivityTimeBucket,
};

fn total_minutes(rows: &[ActivityBucket]) -> i64 {
    rows.iter().map(|row| row.minutes).sum()
}

fn bucket_strategy() -> impl Strategy<Value = ActivityTimeBucket> {
    prop_oneof![
        Just(ActivityTimeBucket::Year),
        Just(ActivityTimeBucket::Month),
        Just(ActivityTimeBucket::Week),
        Just(ActivityTimeBucket::DayOfWeek),
        Just(ActivityTimeBucket::HourOfDay),
    ]
}

proptest! {
    #[test]
    fn minute_aligned_timeline_preserves_total_minutes(
        sessions in minute_aligned_sessions(32),
        bucket in bucket_strategy(),
        offset_minutes in -840i64..=840,
    ) {
        let expected: i64 = sessions
            .iter()
            .map(|(start, end)| (end - start) / MINUTE_MS)
            .sum();
        let rows = activity_timeline(&sessions, bucket, offset_minutes, None, None);

        prop_assert_eq!(total_minutes(&rows), expected);
    }

    #[test]
    fn timeline_is_independent_of_session_order(
        sessions in minute_aligned_sessions(32),
        bucket in bucket_strategy(),
        offset_minutes in -840i64..=840,
    ) {
        let expected = activity_timeline(&sessions, bucket, offset_minutes, None, None);
        let mut reversed = sessions;
        reversed.reverse();

        prop_assert_eq!(
            activity_timeline(&reversed, bucket, offset_minutes, None, None),
            expected
        );
    }

    #[test]
    fn narrowing_timeline_window_never_increases_minutes(
        sessions in minute_aligned_sessions(32),
        from_minutes in 0i64..=30 * 24 * 60,
        width_minutes in 0i64..=30 * 24 * 60,
        bucket in bucket_strategy(),
        offset_minutes in -840i64..=840,
    ) {
        let from = BASE_MS + from_minutes * MINUTE_MS;
        let to = from + width_minutes * MINUTE_MS;
        let all_rows = activity_timeline(&sessions, bucket, offset_minutes, None, None);
        let window_rows =
            activity_timeline(&sessions, bucket, offset_minutes, Some(from), Some(to));

        prop_assert!(total_minutes(&window_rows) <= total_minutes(&all_rows));
    }

    #[test]
    fn histogram_buckets_are_complete_and_unique(
        sessions in minute_aligned_sessions(16),
        offset_minutes in -840i64..=840,
    ) {
        let weekdays = activity_timeline(
            &sessions,
            ActivityTimeBucket::DayOfWeek,
            offset_minutes,
            None,
            None,
        );
        let hours = activity_timeline(
            &sessions,
            ActivityTimeBucket::HourOfDay,
            offset_minutes,
            None,
            None,
        );
        let weekday_keys: BTreeSet<_> = weekdays.iter().map(|row| row.key.as_str()).collect();
        let hour_keys: BTreeSet<_> = hours.iter().map(|row| row.key.as_str()).collect();

        prop_assert_eq!(weekdays.len(), 7);
        prop_assert_eq!(weekday_keys.len(), 7);
        prop_assert_eq!(hours.len(), 24);
        prop_assert_eq!(hour_keys.len(), 24);
    }

    #[test]
    fn streaks_are_independent_of_session_order(
        sessions in minute_aligned_sessions(32),
        now_offset_minutes in 0i64..=90 * 24 * 60,
        offset_minutes in -840i64..=840,
    ) {
        let now_ms = BASE_MS + now_offset_minutes * MINUTE_MS;
        let expected = activity_streaks(&sessions, now_ms, offset_minutes);
        let mut reversed = sessions;
        reversed.reverse();

        prop_assert_eq!(
            activity_streaks(&reversed, now_ms, offset_minutes),
            expected
        );
    }

    #[test]
    fn streak_totals_match_valid_input_intervals(
        sessions in prop::collection::vec(
            (
                BASE_MS..=BASE_MS + 60 * 24 * 60 * MINUTE_MS,
                BASE_MS..=BASE_MS + 60 * 24 * 60 * MINUTE_MS,
            ),
            0..=32,
        ),
        now_offset_minutes in 0i64..=90 * 24 * 60,
        offset_minutes in -840i64..=840,
    ) {
        let valid: Vec<_> = sessions
            .iter()
            .copied()
            .filter(|(start, end)| end > start)
            .collect();
        let expected_ms: i64 = valid.iter().map(|(start, end)| end - start).sum();
        let now_ms = BASE_MS + now_offset_minutes * MINUTE_MS;
        let streaks = activity_streaks(&sessions, now_ms, offset_minutes);

        prop_assert_eq!(streaks.session_count, valid.len());
        prop_assert_eq!(streaks.total_minutes, expected_ms / MINUTE_MS);
        prop_assert!(streaks.longest_break_days >= 0);
        prop_assert!(streaks.current_break_days >= 0);
        prop_assert!(streaks.longest_play_streak_days >= 0);
        prop_assert!(streaks.total_active_days >= 0);
    }
}
