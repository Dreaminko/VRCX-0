use serde_json::Value;
use vrcx_0_persistence::game_log::GameLogWriteBatch;

use crate::{GameLogProjection, OverlayActivitySnapshot, RuntimeEventBus};

pub trait RuntimeGameEventBusExt {
    fn emit_game_log_side_effect(&self, kind: &str, payload: Value);
    fn emit_game_client_event(&self, kind: &str, payload: Value);
    fn emit_runtime_game_log_event(&self, raw: Vec<String>);
    fn emit_game_log_projection(&self, projection: GameLogProjection);
    fn emit_game_log_persistence_fallback(
        &self,
        batch: &GameLogWriteBatch,
        raw_rows: Vec<Vec<String>>,
        error: &str,
    );
    fn emit_runtime_worker_error(&self, worker: &str, message: &str);
    fn emit_overlay_activity_snapshot(&self, payload: OverlayActivitySnapshot);
}

impl RuntimeGameEventBusExt for RuntimeEventBus {
    fn emit_game_log_side_effect(&self, kind: &str, payload: Value) {
        self.emit(
            "gameLogSideEffect",
            serde_json::json!({
                "kind": kind,
                "payload": payload,
            }),
        );
    }

    fn emit_game_client_event(&self, kind: &str, payload: Value) {
        self.emit(
            "gameClientEvent",
            serde_json::json!({
                "kind": kind,
                "payload": payload,
            }),
        );
    }

    fn emit_runtime_game_log_event(&self, raw: Vec<String>) {
        self.emit(
            "runtimeGameLogEvent",
            serde_json::json!({
                "runtimePersisted": true,
                "raw": raw,
            }),
        );
    }

    fn emit_game_log_projection(&self, projection: GameLogProjection) {
        self.emit("gameLogProjection", projection);
    }

    fn emit_game_log_persistence_fallback(
        &self,
        batch: &GameLogWriteBatch,
        raw_rows: Vec<Vec<String>>,
        error: &str,
    ) {
        // Compatibility event name. This is telemetry-only; the WebView must not
        // write the batch as a fallback for runtime-originated GameLog events.
        self.emit(
            "gameLogPersistenceFallback",
            serde_json::json!({
                "batch": batch,
                "rawRows": raw_rows,
                "error": error,
            }),
        );
    }

    fn emit_runtime_worker_error(&self, worker: &str, message: &str) {
        self.emit(
            "runtimeWorkerError",
            serde_json::json!({
                "worker": worker,
                "message": message,
            }),
        );
    }

    fn emit_overlay_activity_snapshot(&self, payload: OverlayActivitySnapshot) {
        self.emit("overlayActivitySnapshot", payload);
    }
}
