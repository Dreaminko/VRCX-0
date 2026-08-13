use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
#[cfg(any(test, feature = "test-utils"))]
use std::time::Instant;

use crate::RuntimeEventBus;
use crate::RuntimeGameEventBusExt;
use crate::RuntimeWorkerErrorPayload;
use crate::{Error, Result};

const DEFAULT_CAPACITY: usize = 8192;
const DEFAULT_MAX_BATCH: usize = 256;
const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverflowPolicy {
    Backpressure,
    DropOldest,
}

#[derive(Clone, Debug)]
pub struct RuntimeWorkerOptions {
    pub capacity: usize,
    pub max_batch: usize,
    pub flush_interval: Duration,
    pub overflow_policy: OverflowPolicy,
}

impl Default for RuntimeWorkerOptions {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_CAPACITY,
            max_batch: DEFAULT_MAX_BATCH,
            flush_interval: DEFAULT_FLUSH_INTERVAL,
            overflow_policy: OverflowPolicy::Backpressure,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimePushReport {
    pub accepted: usize,
    pub dropped: usize,
}

pub trait RuntimeJobHandler<T>: Send + Sync + 'static {
    fn handle_batch(&self, batch: Vec<T>) -> Result<()>;
}

impl<T, F> RuntimeJobHandler<T> for F
where
    F: Fn(Vec<T>) -> Result<()> + Send + Sync + 'static,
{
    fn handle_batch(&self, batch: Vec<T>) -> Result<()> {
        self(batch)
    }
}

pub struct RuntimeWorker<T>
where
    T: Send + 'static,
{
    inner: Arc<WorkerInner<T>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

struct WorkerInner<T>
where
    T: Send + 'static,
{
    name: String,
    options: RuntimeWorkerOptions,
    state: Mutex<WorkerState<T>>,
    available: Condvar,
    not_full: Condvar,
    idle: Condvar,
    handler: Arc<dyn RuntimeJobHandler<T>>,
    event_bus: RuntimeEventBus,
}

struct WorkerState<T> {
    queue: VecDeque<T>,
    active_batches: usize,
    closed: bool,
}

impl<T> RuntimeWorker<T>
where
    T: Send + 'static,
{
    pub fn start(
        name: impl Into<String>,
        options: RuntimeWorkerOptions,
        event_bus: RuntimeEventBus,
        handler: impl RuntimeJobHandler<T>,
    ) -> Self {
        let mut options = options;
        options.capacity = options.capacity.max(1);
        options.max_batch = options.max_batch.max(1);

        let inner = Arc::new(WorkerInner {
            name: name.into(),
            options,
            state: Mutex::new(WorkerState {
                queue: VecDeque::new(),
                active_batches: 0,
                closed: false,
            }),
            available: Condvar::new(),
            not_full: Condvar::new(),
            idle: Condvar::new(),
            handler: Arc::new(handler),
            event_bus,
        });
        let thread_inner = Arc::clone(&inner);
        let handle = thread::Builder::new()
            .name(format!("{}-worker", inner.name))
            .spawn(move || worker_loop(thread_inner))
            .expect("failed to spawn runtime worker");

        Self {
            inner,
            handle: Mutex::new(Some(handle)),
        }
    }

    pub fn push_batch(&self, batch: impl IntoIterator<Item = T>) -> Result<RuntimePushReport> {
        let mut report = RuntimePushReport::default();
        let mut state =
            self.inner.state.lock().map_err(|error| {
                Error::Custom(format!("{} worker lock: {error}", self.inner.name))
            })?;

        for item in batch {
            while state.queue.len() >= self.inner.options.capacity {
                match self.inner.options.overflow_policy {
                    OverflowPolicy::Backpressure => {
                        if state.closed {
                            return Err(Error::Custom(format!(
                                "{} worker is stopped",
                                self.inner.name
                            )));
                        }
                        state = self.inner.not_full.wait(state).map_err(|error| {
                            Error::Custom(format!("{} worker wait: {error}", self.inner.name))
                        })?;
                    }
                    OverflowPolicy::DropOldest => {
                        state.queue.pop_front();
                        report.dropped += 1;
                        break;
                    }
                }
            }

            if state.closed {
                return Err(Error::Custom(format!(
                    "{} worker is stopped",
                    self.inner.name
                )));
            }
            state.queue.push_back(item);
            report.accepted += 1;
            self.inner.available.notify_one();
        }

        Ok(report)
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn wait_until_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut state = match self.inner.state.lock() {
            Ok(state) => state,
            Err(_) => return false,
        };

        while !state.queue.is_empty() || state.active_batches > 0 {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let wait_for = deadline.saturating_duration_since(now);
            let Ok((next_state, result)) = self.inner.idle.wait_timeout(state, wait_for) else {
                return false;
            };
            state = next_state;
            if result.timed_out() && (!state.queue.is_empty() || state.active_batches > 0) {
                return false;
            }
        }
        true
    }

    pub fn stop(&self) {
        {
            let Ok(mut state) = self.inner.state.lock() else {
                return;
            };
            state.closed = true;
            self.inner.available.notify_all();
            self.inner.not_full.notify_all();
        }

        if let Ok(mut handle) = self.handle.lock() {
            if let Some(handle) = handle.take() {
                let _ = handle.join();
            }
        }
    }
}

impl<T> Drop for RuntimeWorker<T>
where
    T: Send + 'static,
{
    fn drop(&mut self) {
        {
            let Ok(mut state) = self.inner.state.lock() else {
                return;
            };
            state.closed = true;
            self.inner.available.notify_all();
            self.inner.not_full.notify_all();
        }

        if let Ok(mut handle) = self.handle.lock() {
            if let Some(handle) = handle.take() {
                let _ = handle.join();
            }
        }
    }
}

fn worker_loop<T>(inner: Arc<WorkerInner<T>>)
where
    T: Send + 'static,
{
    loop {
        let batch = {
            let mut state = match inner.state.lock() {
                Ok(state) => state,
                Err(error) => {
                    tracing::warn!("{} worker state lock failed: {error}", inner.name);
                    return;
                }
            };

            while state.queue.is_empty() && !state.closed {
                state = match inner.available.wait(state) {
                    Ok(state) => state,
                    Err(error) => {
                        tracing::warn!("{} worker wait failed: {error}", inner.name);
                        return;
                    }
                };
            }

            if state.closed && state.queue.is_empty() {
                inner.idle.notify_all();
                return;
            }

            if !state.closed
                && state.queue.len() < inner.options.max_batch
                && !inner.options.flush_interval.is_zero()
            {
                match inner
                    .available
                    .wait_timeout(state, inner.options.flush_interval)
                {
                    Ok((next_state, _)) => state = next_state,
                    Err(error) => {
                        tracing::warn!("{} worker flush wait failed: {error}", inner.name);
                        return;
                    }
                }
            }

            let take = inner.options.max_batch.min(state.queue.len());
            let batch = state.queue.drain(..take).collect::<Vec<_>>();
            state.active_batches += 1;
            inner.not_full.notify_all();
            batch
        };

        if !batch.is_empty() {
            if let Err(error) = inner.handler.handle_batch(batch) {
                tracing::warn!("{} worker batch failed: {error}", inner.name);
                inner
                    .event_bus
                    .emit_runtime_worker_error(RuntimeWorkerErrorPayload {
                        worker: inner.name.clone(),
                        message: error.to_string(),
                    });
            }
        }

        if let Ok(mut state) = inner.state.lock() {
            state.active_batches = state.active_batches.saturating_sub(1);
            if state.queue.is_empty() && state.active_batches == 0 {
                inner.idle.notify_all();
            }
            inner.not_full.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    use serde_json::json;

    use crate::RuntimeEventBus;
    use crate::{Error, Result};

    use super::{OverflowPolicy, RuntimeWorker, RuntimeWorkerOptions};

    #[test]
    fn processes_batches_in_order() -> Result<()> {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let worker_seen = Arc::clone(&seen);
        let worker = RuntimeWorker::start(
            "test-order",
            RuntimeWorkerOptions {
                max_batch: 2,
                flush_interval: Duration::from_millis(1),
                ..Default::default()
            },
            RuntimeEventBus::new(),
            move |batch: Vec<i32>| {
                worker_seen.lock().unwrap().extend(batch);
                Ok(())
            },
        );

        worker.push_batch([1, 2, 3, 4, 5])?;
        assert!(worker.wait_until_idle(Duration::from_secs(2)));
        assert_eq!(*seen.lock().unwrap(), vec![1, 2, 3, 4, 5]);
        worker.stop();
        Ok(())
    }

    #[test]
    fn drop_oldest_keeps_newest_items() -> Result<()> {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let worker_seen = Arc::clone(&seen);
        let worker = RuntimeWorker::start(
            "test-drop-oldest",
            RuntimeWorkerOptions {
                capacity: 3,
                max_batch: 10,
                flush_interval: Duration::from_millis(100),
                overflow_policy: OverflowPolicy::DropOldest,
            },
            RuntimeEventBus::new(),
            move |batch: Vec<i32>| {
                worker_seen.lock().unwrap().extend(batch);
                Ok(())
            },
        );

        let report = worker.push_batch([1, 2, 3, 4, 5])?;
        assert_eq!(report.accepted, 5);
        assert_eq!(report.dropped, 2);
        assert!(worker.wait_until_idle(Duration::from_secs(2)));
        assert_eq!(*seen.lock().unwrap(), vec![3, 4, 5]);
        worker.stop();
        Ok(())
    }

    #[test]
    fn continues_after_handler_error() -> Result<()> {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let worker_seen = Arc::clone(&seen);
        let event_bus = RuntimeEventBus::new();
        let worker = RuntimeWorker::start(
            "test-error",
            RuntimeWorkerOptions {
                max_batch: 1,
                flush_interval: Duration::from_millis(1),
                ..Default::default()
            },
            event_bus.clone(),
            move |batch: Vec<i32>| {
                let value = batch[0];
                worker_seen.lock().unwrap().push(value);
                if value == 1 {
                    Err(Error::Custom("expected test error".into()))
                } else {
                    Ok(())
                }
            },
        );

        worker.push_batch([1, 2])?;
        assert!(worker.wait_until_idle(Duration::from_secs(2)));
        assert_eq!(*seen.lock().unwrap(), vec![1, 2]);
        let events = event_bus.take_events_for_test();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "runtimeWorkerError");
        assert_eq!(
            events[0].payload,
            json!({
                "worker": "test-error",
                "message": "expected test error"
            })
        );
        worker.stop();
        Ok(())
    }

    #[test]
    fn wait_until_idle_includes_the_active_handler() -> Result<()> {
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let release_rx = Mutex::new(release_rx);
        let worker = RuntimeWorker::start(
            "test-active-handler",
            RuntimeWorkerOptions {
                max_batch: 1,
                flush_interval: Duration::ZERO,
                ..Default::default()
            },
            RuntimeEventBus::new(),
            move |_batch: Vec<i32>| {
                started_tx.send(()).unwrap();
                release_rx.lock().unwrap().recv().unwrap();
                Ok(())
            },
        );

        worker.push_batch([1])?;
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!worker.wait_until_idle(Duration::from_millis(20)));

        release_tx.send(()).unwrap();
        assert!(worker.wait_until_idle(Duration::from_secs(2)));
        worker.stop();
        Ok(())
    }

    #[test]
    fn backpressure_waits_for_capacity_without_dropping_work() -> Result<()> {
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let release_rx = Mutex::new(release_rx);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let worker_seen = Arc::clone(&seen);
        let worker = Arc::new(RuntimeWorker::start(
            "test-backpressure",
            RuntimeWorkerOptions {
                capacity: 1,
                max_batch: 1,
                flush_interval: Duration::ZERO,
                overflow_policy: OverflowPolicy::Backpressure,
            },
            RuntimeEventBus::new(),
            move |batch: Vec<i32>| {
                let value = batch[0];
                if value == 1 {
                    started_tx.send(()).unwrap();
                    release_rx.lock().unwrap().recv().unwrap();
                }
                worker_seen.lock().unwrap().push(value);
                Ok(())
            },
        ));

        worker.push_batch([1])?;
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.push_batch([2])?;

        let producer_worker = Arc::clone(&worker);
        let (pushed_tx, pushed_rx) = mpsc::sync_channel(1);
        let producer = std::thread::spawn(move || {
            pushed_tx.send(producer_worker.push_batch([3])).unwrap();
        });
        assert!(pushed_rx.recv_timeout(Duration::from_millis(20)).is_err());

        release_tx.send(()).unwrap();
        let report = pushed_rx.recv_timeout(Duration::from_secs(2)).unwrap()?;
        assert_eq!(report.accepted, 1);
        assert_eq!(report.dropped, 0);
        producer.join().unwrap();
        assert!(worker.wait_until_idle(Duration::from_secs(2)));
        assert_eq!(*seen.lock().unwrap(), vec![1, 2, 3]);
        worker.stop();
        Ok(())
    }

    #[test]
    fn stop_drains_already_accepted_work_before_returning() -> Result<()> {
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let release_rx = Mutex::new(release_rx);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let worker_seen = Arc::clone(&seen);
        let worker = RuntimeWorker::start(
            "test-stop-drain",
            RuntimeWorkerOptions {
                max_batch: 1,
                flush_interval: Duration::ZERO,
                ..Default::default()
            },
            RuntimeEventBus::new(),
            move |batch: Vec<i32>| {
                let value = batch[0];
                if value == 1 {
                    started_tx.send(()).unwrap();
                    release_rx.lock().unwrap().recv().unwrap();
                }
                worker_seen.lock().unwrap().push(value);
                Ok(())
            },
        );

        worker.push_batch([1, 2])?;
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let (stopped_tx, stopped_rx) = mpsc::sync_channel(1);
        let stopper = std::thread::spawn(move || {
            worker.stop();
            stopped_tx.send(()).unwrap();
        });
        assert!(stopped_rx.recv_timeout(Duration::from_millis(20)).is_err());

        release_tx.send(()).unwrap();
        stopped_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        stopper.join().unwrap();
        assert_eq!(*seen.lock().unwrap(), vec![1, 2]);
        Ok(())
    }

    #[test]
    fn stop_wakes_a_backpressured_producer_with_a_stopped_error() -> Result<()> {
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let release_rx = Mutex::new(release_rx);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let worker_seen = Arc::clone(&seen);
        let worker = Arc::new(RuntimeWorker::start(
            "test-stop-backpressure",
            RuntimeWorkerOptions {
                capacity: 1,
                max_batch: 1,
                flush_interval: Duration::ZERO,
                overflow_policy: OverflowPolicy::Backpressure,
            },
            RuntimeEventBus::new(),
            move |batch: Vec<i32>| {
                let value = batch[0];
                if value == 1 {
                    started_tx.send(()).unwrap();
                    release_rx.lock().unwrap().recv().unwrap();
                }
                worker_seen.lock().unwrap().push(value);
                Ok(())
            },
        ));

        worker.push_batch([1])?;
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.push_batch([2])?;

        let producer_worker = Arc::clone(&worker);
        let (pushed_tx, pushed_rx) = mpsc::sync_channel(1);
        let producer = std::thread::spawn(move || {
            pushed_tx.send(producer_worker.push_batch([3])).unwrap();
        });
        assert!(pushed_rx.recv_timeout(Duration::from_millis(20)).is_err());

        let stopper_worker = Arc::clone(&worker);
        let (stopped_tx, stopped_rx) = mpsc::sync_channel(1);
        let stopper = std::thread::spawn(move || {
            stopper_worker.stop();
            stopped_tx.send(()).unwrap();
        });
        let error = pushed_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("test-stop-backpressure worker is stopped"));

        release_tx.send(()).unwrap();
        stopped_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        producer.join().unwrap();
        stopper.join().unwrap();
        assert_eq!(*seen.lock().unwrap(), vec![1, 2]);
        Ok(())
    }
}
