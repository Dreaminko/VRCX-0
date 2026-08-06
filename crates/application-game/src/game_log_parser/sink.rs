use vrcx_0_core::game_log_parser::{convert_log_time_to_iso8601, GameLogEventKind, ParsedLogEntry};

pub(crate) trait GameLogParseSink {
    fn push(&mut self, entry: ParsedLogEntry);

    fn set_vrc_closed_gracefully(&mut self, value: bool);

    fn push_event(&mut self, file_name: &str, line: &str, kind: GameLogEventKind) {
        self.push(ParsedLogEntry::new(
            file_name,
            convert_log_time_to_iso8601(line),
            kind,
        ));
    }
}
