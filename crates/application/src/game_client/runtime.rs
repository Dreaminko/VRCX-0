use std::sync::{Arc, Mutex};
#[cfg(any(test, feature = "test-utils"))]
use std::time::Duration;

use vrcx_0_persistence::config::ConfigRepository;
use vrcx_0_persistence::DatabaseService;

use crate::event_bus::RuntimeEventBus;
use crate::process_monitor::GameProcessEvent;
use crate::session::HostSessionRuntime;
use crate::task_supervisor::TaskSupervisor;
use crate::worker::{RuntimeWorker, RuntimeWorkerOptions};
use crate::Result;

use super::actions::GameClientActions;
use super::processor::{
    GameClientCacheActions, GameClientJob, GameClientLocationSource, GameClientProcessor,
    GameClientProcessorDeps, GameClientState, GameClientWindowActions,
};

#[derive(Clone)]
pub struct GameClientRuntimeDeps {
    pub db: Arc<DatabaseService>,
    pub config: ConfigRepository,
    pub event_bus: RuntimeEventBus,
    pub tasks: TaskSupervisor,
    pub session: HostSessionRuntime,
    pub actions: Arc<dyn GameClientActions>,
    pub cache_actions: Arc<dyn GameClientCacheActions>,
    pub location_source: Arc<dyn GameClientLocationSource>,
    pub window_actions: Arc<dyn GameClientWindowActions>,
}

pub struct GameClientRuntime {
    state: Arc<Mutex<GameClientState>>,
    worker: RuntimeWorker<GameClientJob>,
}

impl GameClientRuntime {
    pub fn new(deps: GameClientRuntimeDeps) -> Self {
        let state = Arc::new(Mutex::new(GameClientState::default()));
        let processor = GameClientProcessor::new(
            GameClientProcessorDeps {
                db: deps.db,
                config: deps.config,
                event_bus: deps.event_bus.clone(),
                tasks: deps.tasks,
                session: deps.session,
                actions: deps.actions,
                cache_actions: deps.cache_actions,
                location_source: deps.location_source,
                window_actions: deps.window_actions,
            },
            Arc::clone(&state),
        );
        let worker_processor = processor.clone();
        let worker = RuntimeWorker::start(
            "game-client",
            RuntimeWorkerOptions::default(),
            deps.event_bus,
            move |jobs| worker_processor.handle_jobs(jobs),
        );

        Self { state, worker }
    }

    pub fn set_runtime_state(&self, current_location: &str) {
        let Ok(mut state) = self.state.lock() else {
            tracing::warn!("failed to lock GameClient runtime state");
            return;
        };
        state.current_location = current_location.trim().to_string();
    }

    pub fn on_game_process_event(&self, event: GameProcessEvent) -> Result<()> {
        if event.game_changed && !event.is_game_running {
            self.enqueue_job(GameClientJob::GameStopped)?;
        }
        Ok(())
    }

    pub fn stop(&self) {
        self.worker.stop();
    }

    fn enqueue_job(&self, job: GameClientJob) -> Result<()> {
        self.worker.push_batch([job])?;
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn wait_until_idle(&self) -> bool {
        self.worker.wait_until_idle(Duration::from_secs(2))
    }
}
