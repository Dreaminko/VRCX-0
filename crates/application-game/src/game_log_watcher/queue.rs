use vrcx_0_core::game_log_parser::ParsedLogEntry;

use crate::game_log_parser::GameLogParseSink;

use super::sink::GameLogEventOrigin;
use super::watcher::Inner;

const MAX_COMPAT_LOG_ROWS: usize = 5000;

pub(super) struct WatcherParseSink<'a> {
    pub(super) inner: &'a Inner,
    pub(super) first_run: bool,
}

impl GameLogParseSink for WatcherParseSink<'_> {
    fn push(&mut self, entry: ParsedLogEntry) {
        let inner = self.inner;
        if inner.event_sink.is_some() {
            inner.event_buffer.lock().unwrap().push(entry.event);
        }

        if !self.first_run {
            if let Ok(json) = serde_json::to_string(&entry.compat_row) {
                inner.compat_event_buffer.lock().unwrap().push(json);
            }
        }
        let mut log_list = inner.log_list.write().unwrap();
        log_list.push(entry.compat_row);
        if log_list.len() > MAX_COMPAT_LOG_ROWS {
            let overflow = log_list.len() - MAX_COMPAT_LOG_ROWS;
            log_list.drain(..overflow);
        }
    }

    fn set_vrc_closed_gracefully(&mut self, value: bool) {
        *self.inner.vrc_closed_gracefully.lock().unwrap() = value;
    }
}

pub(super) fn flush_game_log_events(inner: &Inner, first_run: bool) {
    let Some(event_sink) = &inner.event_sink else {
        return;
    };

    let events = {
        let mut buffer = inner.event_buffer.lock().unwrap();
        if buffer.is_empty() {
            return;
        }
        std::mem::take(&mut *buffer)
    };

    let origin = if first_run {
        GameLogEventOrigin::InitialScan
    } else {
        GameLogEventOrigin::Live
    };
    if let Err(error) = event_sink.ingest_game_log_events_with_origin(&events, origin) {
        tracing::warn!("failed to ingest GameLog event batch in runtime: {error}");
    }
}
