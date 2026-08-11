use std::collections::VecDeque;
use std::sync::atomic::Ordering;

use serde_json::Value;
use vrcx_0_application_core::{Error, Result};
use vrcx_0_core::json::RawJson;
use vrcx_0_persistence::feed::{
    feed_latest_query, feed_live_search_query, feed_search_query, FeedLatestQueryInput,
    FeedLiveEntryInput, FeedLiveQueryMatcher, FeedReadModelOutput, FeedRowOutput,
    FeedSearchQueryInput,
};

use crate::realtime::{
    RealtimeEntryCorrectionFields, RealtimeFeedPatch, RealtimeFeedProjection, RealtimeFeedUpsert,
};
use crate::world_enrich::feed_entry_correction_id;

use super::RealtimeHostRuntime;

const FEED_LIVE_CACHE_MAX_ENTRIES: usize = 10_000;

#[derive(Clone, Debug)]
struct CachedFeedEntry {
    sequence: i64,
    entry: Value,
}

#[derive(Default)]
pub(super) struct FeedLiveCache {
    owner_user_id: String,
    sequence: i64,
    entries: VecDeque<CachedFeedEntry>,
}

impl FeedLiveCache {
    fn reset(&mut self) {
        self.owner_user_id.clear();
        self.sequence = 0;
        self.entries.clear();
    }

    fn prepare_owner(&mut self, owner_user_id: &str) {
        if self.owner_user_id != owner_user_id {
            self.reset();
            self.owner_user_id = owner_user_id.to_string();
        }
    }

    fn push_entries(
        &mut self,
        owner_user_id: &str,
        entries: Vec<Value>,
    ) -> Vec<RealtimeFeedUpsert> {
        self.prepare_owner(owner_user_id);
        let mut upserts = Vec::new();
        for mut entry in entries {
            let Some(object) = entry.as_object_mut() else {
                continue;
            };
            object.insert(
                "ownerUserId".into(),
                Value::String(owner_user_id.to_string()),
            );
            self.sequence = self.sequence.saturating_add(1);
            let sequence = self.sequence;
            self.entries.push_back(CachedFeedEntry {
                sequence,
                entry: entry.clone(),
            });
            upserts.push(RealtimeFeedUpsert {
                sequence,
                entry: RawJson::from(entry),
            });
        }
        while self.entries.len() > FEED_LIVE_CACHE_MAX_ENTRIES {
            self.entries.pop_front();
        }
        upserts
    }

    fn patch_entry(
        &mut self,
        owner_user_id: &str,
        id: &str,
        fields: &RealtimeEntryCorrectionFields,
    ) -> Option<i64> {
        if self.owner_user_id != owner_user_id {
            return None;
        }
        let mut changed_indices = Vec::new();
        for (index, entry) in self.entries.iter_mut().enumerate() {
            let Some(object) = entry.entry.as_object_mut() else {
                continue;
            };
            if feed_entry_correction_id(object) != id {
                continue;
            }
            let mut changed = false;
            for (key, value) in [
                ("displayName", fields.display_name.as_ref()),
                ("worldName", fields.world_name.as_ref()),
                ("displayLocation", fields.display_location.as_ref()),
            ] {
                let Some(value) = value else {
                    continue;
                };
                if object.get(key).and_then(Value::as_str) != Some(value) {
                    object.insert(key.into(), Value::String(value.clone()));
                    changed = true;
                }
            }
            if changed {
                changed_indices.push(index);
            }
        }
        if changed_indices.is_empty() {
            return None;
        }
        self.sequence = self.sequence.saturating_add(1);
        for index in changed_indices {
            if let Some(entry) = self.entries.get_mut(index) {
                entry.sequence = self.sequence;
            }
        }
        Some(self.sequence)
    }

    fn snapshot_matching(
        &self,
        owner_user_id: &str,
        matcher: &FeedLiveQueryMatcher,
    ) -> (Vec<FeedLiveEntryInput>, i64) {
        if self.owner_user_id != owner_user_id {
            return (Vec::new(), 0);
        }
        let mut entries = self
            .entries
            .iter()
            .rev()
            .filter(|entry| matcher.matches(&entry.entry))
            .take(matcher.max_rows().unwrap_or(usize::MAX))
            .map(|entry| FeedLiveEntryInput {
                sequence: entry.sequence,
                entry: RawJson::from(entry.entry.clone()),
            })
            .collect::<Vec<_>>();
        entries.reverse();
        (entries, self.sequence)
    }
}

impl RealtimeHostRuntime {
    pub(super) fn emit_feed_entries(
        &self,
        generation: u64,
        owner_user_id: &str,
        entries: Vec<Value>,
    ) {
        if entries.is_empty() {
            return;
        }
        let _owner = self
            .feed_owner_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self
            .active_feed_scope()
            .is_none_or(|scope| scope != (generation, owner_user_id.to_string()))
        {
            return;
        }
        let upserts = self
            .feed_live_cache
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push_entries(owner_user_id, entries);
        if upserts.is_empty() {
            return;
        }
        self.deps.event_bus.emit(RealtimeFeedProjection {
            generation,
            owner_user_id: owner_user_id.to_string(),
            upserts,
            patches: Vec::new(),
        });
    }

    pub(super) fn emit_feed_patch(&self, id: String, fields: RealtimeEntryCorrectionFields) {
        let _owner = self
            .feed_owner_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some((generation, owner_user_id)) = self.active_feed_scope() else {
            return;
        };
        let sequence = self
            .feed_live_cache
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .patch_entry(&owner_user_id, &id, &fields);
        let Some(sequence) = sequence else {
            return;
        };
        self.deps.event_bus.emit(RealtimeFeedProjection {
            generation,
            owner_user_id,
            upserts: Vec::new(),
            patches: vec![RealtimeFeedPatch {
                sequence,
                id,
                fields,
            }],
        });
    }

    pub fn query_feed_latest(&self, query: FeedLatestQueryInput) -> Result<FeedReadModelOutput> {
        let matcher = FeedLiveQueryMatcher::for_latest(&query);
        let (live_entries, watermark) = self.feed_live_snapshot(&query.user_id, &matcher)?;
        feed_latest_query(
            self.deps.db.as_ref(),
            query,
            live_entries,
            watermark,
            !self.feed_persistence_disabled.load(Ordering::Relaxed),
        )
        .map_err(Error::from)
    }

    pub fn query_feed_search(&self, query: FeedSearchQueryInput) -> Result<Vec<FeedRowOutput>> {
        if self.feed_persistence_disabled.load(Ordering::Relaxed) {
            let matcher = FeedLiveQueryMatcher::for_search(&query);
            let (live_entries, watermark) = self.feed_live_snapshot(&query.user_id, &matcher)?;
            return Ok(feed_live_search_query(query, live_entries, watermark).rows);
        }
        feed_search_query(self.deps.db.as_ref(), query).map_err(Error::from)
    }

    pub(super) fn reset_feed_live_cache(&self) {
        let _owner = self
            .feed_owner_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.feed_live_cache
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .reset();
    }

    fn feed_live_snapshot(
        &self,
        owner_user_id: &str,
        matcher: &FeedLiveQueryMatcher,
    ) -> Result<(Vec<FeedLiveEntryInput>, i64)> {
        self.feed_live_cache
            .lock()
            .map(|cache| cache.snapshot_matching(owner_user_id, matcher))
            .map_err(|error| Error::Custom(format!("feed live cache lock: {error}")))
    }

    fn active_feed_scope(&self) -> Option<(u64, String)> {
        self.state.lock().ok().and_then(|state| {
            state
                .connection
                .active_context
                .as_ref()
                .map(|active| (active.generation, active.session.user_id.trim().to_string()))
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vrcx_0_persistence::feed::{FeedFilter, FeedLatestQueryInput, FeedLiveQueryMatcher};

    use super::FeedLiveCache;
    use crate::realtime::RealtimeEntryCorrectionFields;

    fn latest_matcher(
        user_id: &str,
        filters: Vec<FeedFilter>,
        max_rows: i64,
    ) -> FeedLiveQueryMatcher {
        FeedLiveQueryMatcher::for_latest(&FeedLatestQueryInput {
            user_id: user_id.into(),
            filters,
            favorite_user_ids: Vec::new(),
            scoped_user_ids: Vec::new(),
            excluded_user_ids: Vec::new(),
            favorites_only: false,
            max_rows,
        })
    }

    #[test]
    fn cache_owns_sequences_entries_and_corrections() {
        let mut cache = FeedLiveCache::default();
        let upserts = cache.push_entries(
            "usr_self",
            vec![
                json!({
                    "id": "first",
                    "type": "GPS",
                    "worldName": "wrld_1"
                }),
                json!({ "id": "second", "type": "Online" }),
            ],
        );

        assert_eq!(upserts[0].sequence, 1);
        assert_eq!(upserts[1].sequence, 2);
        assert_eq!(upserts[0].entry.as_value()["ownerUserId"], "usr_self");

        let correction_sequence = cache.patch_entry(
            "usr_self",
            "id:first",
            &RealtimeEntryCorrectionFields {
                display_name: None,
                world_name: Some("Resolved World".into()),
                display_location: Some("Resolved World".into()),
            },
        );
        assert_eq!(correction_sequence, Some(3));

        let matcher = latest_matcher("usr_self", Vec::new(), 10);
        let (entries, watermark) = cache.snapshot_matching("usr_self", &matcher);
        assert_eq!(watermark, 3);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence, 3);
        assert_eq!(entries[0].entry.as_value()["worldName"], "Resolved World");
        assert_eq!(entries[1].sequence, 2);
    }

    #[test]
    fn changing_owner_resets_the_cache_and_sequence() {
        let mut cache = FeedLiveCache::default();
        cache.push_entries("usr_first", vec![json!({ "id": "old", "type": "GPS" })]);

        let upserts = cache.push_entries("usr_second", vec![json!({ "id": "new", "type": "GPS" })]);
        let matcher = latest_matcher("usr_second", Vec::new(), 10);
        let (entries, watermark) = cache.snapshot_matching("usr_second", &matcher);

        assert_eq!(upserts[0].sequence, 1);
        assert_eq!(watermark, 1);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry.as_value()["id"], "new");
        assert!(cache.snapshot_matching("usr_first", &matcher).0.is_empty());
    }

    #[test]
    fn snapshot_clones_only_the_newest_matching_rows() {
        let mut cache = FeedLiveCache::default();
        cache.push_entries(
            "usr_self",
            vec![
                json!({ "id": "gps-old", "type": "GPS" }),
                json!({ "id": "status", "type": "Status" }),
                json!({ "id": "gps-middle", "type": "GPS" }),
                json!({ "id": "gps-new", "type": "GPS" }),
            ],
        );
        let matcher = latest_matcher("usr_self", vec![FeedFilter::Gps], 2);

        let (entries, watermark) = cache.snapshot_matching("usr_self", &matcher);

        assert_eq!(watermark, 4);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry.as_value()["id"], "gps-middle");
        assert_eq!(entries[1].entry.as_value()["id"], "gps-new");
    }
}
