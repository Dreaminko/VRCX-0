#![allow(dead_code)]

use proptest::prelude::*;
use vrcx_0_core::activity_sessions::{ActivitySession, PresenceKind};

pub const BASE_MS: i64 = 1_735_689_600_000;
pub const MINUTE_MS: i64 = 60_000;

pub fn adversarial_text() -> BoxedStrategy<String> {
    let fixed = prop::sample::select(vec![
        String::new(),
        " ".to_string(),
        "\0".to_string(),
        "\n\r\t".to_string(),
        "\u{202e}name".to_string(),
        "\u{2066}name\u{2069}".to_string(),
        "\u{200b}".to_string(),
        "e\u{301}".to_string(),
        "👨‍👩‍👧‍👦".to_string(),
        "名字".to_string(),
        "اسم".to_string(),
        "%_\\'\"".to_string(),
        "CON".to_string(),
        "..\\..\\name".to_string(),
        "x".repeat(4096),
    ]);

    prop_oneof![
        6 => prop::collection::vec(any::<char>(), 0..=96)
            .prop_map(|characters| characters.into_iter().collect()),
        3 => fixed,
        1 => (any::<char>(), 97usize..=512)
            .prop_map(|(character, length)| character.to_string().repeat(length)),
    ]
    .boxed()
}

pub fn meaningful_text() -> BoxedStrategy<String> {
    adversarial_text()
        .prop_map(|value| format!("name:{value}"))
        .boxed()
}

pub fn user_id() -> BoxedStrategy<String> {
    any::<u64>()
        .prop_map(|value| format!("usr_{value}"))
        .boxed()
}

pub fn valid_session() -> BoxedStrategy<ActivitySession> {
    (
        0i64..=30 * 24 * 60 * MINUTE_MS,
        0i64..=12 * 60 * MINUTE_MS,
        any::<bool>(),
        prop_oneof![Just(String::new()), meaningful_text()],
    )
        .prop_map(
            |(start_offset, duration, is_open_tail, source_revision)| ActivitySession {
                start: BASE_MS + start_offset,
                end: BASE_MS + start_offset + duration,
                is_open_tail,
                source_revision,
            },
        )
        .boxed()
}

pub fn valid_sessions(max_count: usize) -> BoxedStrategy<Vec<ActivitySession>> {
    prop::collection::vec(valid_session(), 0..=max_count).boxed()
}

pub fn presence_events() -> BoxedStrategy<Vec<(i64, PresenceKind)>> {
    prop::collection::vec((0i64..=10 * MINUTE_MS, 0u8..3), 0..=64)
        .prop_map(|steps| {
            let mut timestamp = BASE_MS;
            steps
                .into_iter()
                .map(|(gap, kind)| {
                    timestamp += gap;
                    let kind = match kind {
                        0 => PresenceKind::Online,
                        1 => PresenceKind::Offline,
                        _ => PresenceKind::Other,
                    };
                    (timestamp, kind)
                })
                .collect()
        })
        .boxed()
}

pub fn minute_aligned_sessions(max_count: usize) -> BoxedStrategy<Vec<(i64, i64)>> {
    prop::collection::vec((0i64..=60 * 24 * 60, 1i64..=12 * 60), 0..=max_count)
        .prop_map(|values| {
            values
                .into_iter()
                .map(|(start_minute, duration_minutes)| {
                    let start = BASE_MS + start_minute * MINUTE_MS;
                    (start, start + duration_minutes * MINUTE_MS)
                })
                .collect()
        })
        .boxed()
}
