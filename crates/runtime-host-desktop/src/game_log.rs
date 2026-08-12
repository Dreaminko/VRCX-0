use std::sync::Arc;

use crate::log_watcher::{GameLogEvent, GameLogEventOrigin, GameLogEventSink, LogWatcher};
use crate::{HostFileAccess, Result};
use vrcx_0_application_activity::OverlayActivityRuntime;
use vrcx_0_application_core::Error as RuntimeError;
use vrcx_0_application_core::Result as RuntimeResult;
use vrcx_0_application_core::{GameProcessEvent, GameProcessEventSink, InstanceRosterObserver};
use vrcx_0_application_game::{
    GameLogHostActions, GameLogRuntime, GameLogRuntimeDeps, RuntimeSnapshot,
};
use vrcx_0_host::app_paths::AppPaths;
use vrcx_0_host_desktop::{clipboard, game_launch, vrchat_paths};
use vrcx_0_runtime_host::RuntimeHostContext;

fn host_error(error: vrcx_0_host::Error) -> RuntimeError {
    match error {
        vrcx_0_host::Error::Io(error) => RuntimeError::Io(error),
        vrcx_0_host::Error::Json(error) => RuntimeError::Json(error),
        vrcx_0_host::Error::Custom(message) => RuntimeError::Custom(message),
    }
}

struct HostGameLogActions {
    file_access: HostFileAccess,
    app_paths: AppPaths,
}

impl GameLogHostActions for HostGameLogActions {
    fn quit_game(&self) -> i64 {
        i64::from(game_launch::quit_game())
    }

    fn copy_image_to_clipboard(&self, path: &str) -> RuntimeResult<()> {
        clipboard::copy_image_to_clipboard(path).map_err(host_error)
    }

    fn ugc_photo_location(&self, configured_path: Option<String>) -> String {
        let resolved = vrchat_paths::ugc_photo_location(configured_path);
        if self
            .file_access
            .ensure_write_allowed(&resolved, &self.app_paths)
            .is_ok()
        {
            return resolved;
        }
        let fallback = vrchat_paths::ugc_photo_location(None);
        if !fallback.is_empty() {
            tracing::warn!(
                path = %resolved,
                fallback = %fallback,
                "configured UGC path is not authorized; using VRChat photos folder"
            );
        }
        fallback
    }
}

pub struct GameLogHostRuntime {
    context: Arc<RuntimeHostContext>,
    inner: GameLogRuntime,
}

impl GameLogHostRuntime {
    pub fn new(
        context: Arc<RuntimeHostContext>,
        file_access: HostFileAccess,
        app_paths: AppPaths,
        snapshot: Arc<std::sync::Mutex<RuntimeSnapshot>>,
        overlay_activity: OverlayActivityRuntime,
        instance_roster_observer: Option<Arc<dyn InstanceRosterObserver>>,
    ) -> Self {
        let inner = GameLogRuntime::new(GameLogRuntimeDeps {
            db: Arc::clone(&context.db),
            web: Arc::clone(&context.web),
            image_cache: Arc::clone(&context.image_cache),
            event_bus: context.event_bus.clone(),
            tasks: context.tasks.clone(),
            sync: context.sync.clone(),
            auth_scope: context.auth_scope.clone(),
            snapshot,
            session: context.session.clone(),
            overlay_activity,
            world_cache: Arc::clone(&context.world_cache),
            host_actions: Arc::new(HostGameLogActions {
                file_access,
                app_paths,
            }),
            instance_roster_observer,
        });

        Self { context, inner }
    }

    pub fn prime_log_watcher(&self, log_watcher: &LogWatcher) -> Result<()> {
        let last_persisted =
            vrcx_0_persistence::game_log::get_last_game_log_date(&self.context.db)?;
        let resume_after = vrcx_0_persistence::config::get_string(
            &self.context.db,
            "gameLogPersistenceResumeAfter",
            "",
        )?;
        let date_till =
            later_timestamp(&last_persisted, &resume_after).unwrap_or(last_persisted.as_str());
        self.inner.set_persistence_resume_after(&resume_after);
        log_watcher.set_date_till(date_till);
        log_watcher.set_initial_scan_latest_file_only(vrcx_0_persistence::config::get_bool(
            &self.context.db,
            "gameLogDisabled",
            false,
        )?);
        Ok(())
    }

    pub fn set_persistence_disabled(&self, log_watcher: &LogWatcher, disabled: bool) -> Result<()> {
        if self.context.session.snapshot().is_game_running {
            return Err(crate::Error::Custom(
                "VRChat must be closed before changing GameLog history persistence.".into(),
            ));
        }

        if disabled {
            vrcx_0_persistence::config::config_set_values(
                &self.context.db,
                vec![vrcx_0_persistence::config::ConfigWriteEntry {
                    key: "gameLogDisabled".into(),
                    value: "true".into(),
                }],
            )?;
            log_watcher.set_initial_scan_latest_file_only(true);
            return Ok(());
        }

        let resume_after = vrcx_0_core::time::now_iso();
        self.inner.set_persistence_resume_after(&resume_after);
        vrcx_0_persistence::config::config_set_values(
            &self.context.db,
            vec![
                vrcx_0_persistence::config::ConfigWriteEntry {
                    key: "gameLogPersistenceResumeAfter".into(),
                    value: resume_after.clone(),
                },
                vrcx_0_persistence::config::ConfigWriteEntry {
                    key: "gameLogDisabled".into(),
                    value: "false".into(),
                },
            ],
        )?;
        log_watcher.set_date_till(&resume_after);
        log_watcher.set_initial_scan_latest_file_only(false);
        Ok(())
    }

    pub fn stop(&self) {
        self.inner.stop();
    }
}

impl GameLogEventSink for GameLogHostRuntime {
    fn ingest_game_log_event(&self, event: &GameLogEvent) -> RuntimeResult<()> {
        self.inner.ingest_game_log_event(event)
    }

    fn ingest_game_log_events(&self, events: &[GameLogEvent]) -> RuntimeResult<()> {
        self.inner.ingest_game_log_events(events)
    }

    fn ingest_game_log_events_with_origin(
        &self,
        events: &[GameLogEvent],
        origin: GameLogEventOrigin,
    ) -> RuntimeResult<()> {
        self.inner
            .ingest_game_log_events_with_origin(events, origin)
    }
}

fn later_timestamp<'a>(left: &'a str, right: &'a str) -> Option<&'a str> {
    let left_at = vrcx_0_application_game::parse_event_time_ms(left);
    let right_at = vrcx_0_application_game::parse_event_time_ms(right);
    match (left_at, right_at) {
        (Some(left_at), Some(right_at)) => Some(if left_at >= right_at { left } else { right }),
        (Some(_), None) => Some(left),
        (None, Some(_)) => Some(right),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::later_timestamp;

    #[test]
    fn resume_cutoff_only_advances_the_watcher_boundary() {
        assert_eq!(
            later_timestamp("2026-08-06T10:00:00.000Z", "2026-08-06T11:00:00.000Z"),
            Some("2026-08-06T11:00:00.000Z")
        );
        assert_eq!(
            later_timestamp("2026-08-06T12:00:00.000Z", "2026-08-06T11:00:00.000Z"),
            Some("2026-08-06T12:00:00.000Z")
        );
        assert_eq!(
            later_timestamp("2026-08-06T12:00:00.000Z", ""),
            Some("2026-08-06T12:00:00.000Z")
        );
    }
}

impl GameProcessEventSink for GameLogHostRuntime {
    fn on_game_process_event(&self, event: GameProcessEvent) -> RuntimeResult<()> {
        self.inner.on_game_process_event(event)
    }
}
