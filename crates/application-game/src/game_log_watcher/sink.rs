use crate::game_log_parser::GameLogEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameLogEventOrigin {
    Live,
    InitialScan,
}

pub trait GameLogEventSink: Send + Sync {
    fn ingest_game_log_event(&self, event: &GameLogEvent) -> crate::Result<()>;

    fn ingest_game_log_events(&self, events: &[GameLogEvent]) -> crate::Result<()> {
        for event in events {
            self.ingest_game_log_event(event)?;
        }
        Ok(())
    }

    fn ingest_game_log_events_with_origin(
        &self,
        events: &[GameLogEvent],
        _origin: GameLogEventOrigin,
    ) -> crate::Result<()> {
        self.ingest_game_log_events(events)
    }
}
