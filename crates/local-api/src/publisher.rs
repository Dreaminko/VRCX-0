use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::{broadcast, watch};
use vrcx_0_application_core::{InstanceRosterObserver, InstanceRosterSnapshot};

const PUBLISHER_CAPACITY: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalApiInput {
    Roster {
        lifecycle_epoch: u64,
        snapshot: InstanceRosterSnapshot,
    },
    GameRunning {
        lifecycle_epoch: u64,
        running: bool,
    },
}

pub struct LocalApiPublisher {
    roster_sender: broadcast::Sender<RosterEnvelope>,
    lifecycle: Arc<AtomicU64>,
    game_running_sender: watch::Sender<LifecycleState>,
}

pub struct LocalApiInputReceiver {
    roster_receiver: broadcast::Receiver<RosterEnvelope>,
    game_running_receiver: watch::Receiver<LifecycleState>,
    delivered_lifecycle: LifecycleState,
    target_lifecycle: LifecycleState,
    pending_roster: Option<(u64, InstanceRosterSnapshot)>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct LifecycleState {
    epoch: u64,
    running: bool,
}

#[derive(Clone, Debug)]
struct RosterEnvelope {
    lifecycle_epoch: u64,
    snapshot: InstanceRosterSnapshot,
}

pub fn local_api_publisher_channel() -> (LocalApiPublisher, LocalApiInputReceiver) {
    let (roster_sender, roster_receiver) = broadcast::channel(PUBLISHER_CAPACITY);
    let (game_running_sender, game_running_receiver) = watch::channel(LifecycleState::default());
    let lifecycle = Arc::new(AtomicU64::new(0));
    (
        LocalApiPublisher {
            roster_sender,
            lifecycle,
            game_running_sender,
        },
        LocalApiInputReceiver {
            roster_receiver,
            game_running_receiver,
            delivered_lifecycle: LifecycleState::default(),
            target_lifecycle: LifecycleState::default(),
            pending_roster: None,
        },
    )
}

impl LocalApiInputReceiver {
    pub async fn recv(&mut self) -> Option<LocalApiInput> {
        loop {
            let latest_lifecycle = *self.game_running_receiver.borrow_and_update();
            if latest_lifecycle.epoch > self.target_lifecycle.epoch {
                self.target_lifecycle = latest_lifecycle;
            }
            if self.delivered_lifecycle.epoch < self.target_lifecycle.epoch {
                self.delivered_lifecycle = LifecycleState {
                    epoch: self.delivered_lifecycle.epoch.saturating_add(1),
                    running: !self.delivered_lifecycle.running,
                };
                return Some(LocalApiInput::GameRunning {
                    lifecycle_epoch: self.delivered_lifecycle.epoch,
                    running: self.delivered_lifecycle.running,
                });
            }
            if let Some((lifecycle_epoch, snapshot)) = self.pending_roster.take() {
                let current = *self.game_running_receiver.borrow();
                if lifecycle_epoch > current.epoch {
                    self.pending_roster = Some((lifecycle_epoch, snapshot));
                    if self.game_running_receiver.changed().await.is_err() {
                        return None;
                    }
                    continue;
                }
                if current.running
                    && lifecycle_epoch == current.epoch
                    && lifecycle_epoch == self.delivered_lifecycle.epoch
                {
                    return Some(LocalApiInput::Roster {
                        lifecycle_epoch,
                        snapshot,
                    });
                }
            }
            tokio::select! {
                biased;
                changed = self.game_running_receiver.changed() => {
                    if changed.is_err() {
                        return None;
                    }
                }
                roster = self.roster_receiver.recv() => {
                    match roster {
                        Ok(envelope) => {
                            self.pending_roster = Some((
                                envelope.lifecycle_epoch,
                                envelope.snapshot,
                            ));
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => return None,
                    }
                }
            }
        }
    }
}

impl InstanceRosterObserver for LocalApiPublisher {
    fn on_instance_roster(&self, snapshot: InstanceRosterSnapshot) {
        let lifecycle = self.lifecycle.load(Ordering::Acquire);
        if lifecycle & 1 == 0 {
            return;
        }
        let _ = self.roster_sender.send(RosterEnvelope {
            lifecycle_epoch: lifecycle >> 1,
            snapshot,
        });
    }

    fn on_game_running(&self, running: bool) {
        let mut current = self.lifecycle.load(Ordering::Acquire);
        loop {
            if (current & 1 != 0) == running {
                return;
            }
            let epoch = (current >> 1).saturating_add(1);
            let next = (epoch << 1) | u64::from(running);
            match self.lifecycle.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.game_running_sender.send_if_modified(|current| {
                        if current.epoch >= epoch {
                            return false;
                        }
                        *current = LifecycleState { epoch, running };
                        true
                    });
                    return;
                }
                Err(observed) => current = observed,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn full_queue_discards_the_oldest_pending_input() {
        let (publisher, mut receiver) = local_api_publisher_channel();
        publisher.on_game_running(true);
        assert!(matches!(
            receiver.recv().await,
            Some(LocalApiInput::GameRunning { running: true, .. })
        ));
        for index in 0..=PUBLISHER_CAPACITY {
            publisher.on_instance_roster(InstanceRosterSnapshot {
                location: index.to_string(),
                ..Default::default()
            });
        }

        for expected in 1..=PUBLISHER_CAPACITY {
            let Some(LocalApiInput::Roster { snapshot, .. }) = receiver.recv().await else {
                panic!("expected a roster snapshot");
            };
            assert_eq!(snapshot.location, expected.to_string());
        }
    }

    #[tokio::test]
    async fn game_running_transitions_survive_a_roster_burst() {
        let (publisher, mut receiver) = local_api_publisher_channel();
        publisher.on_game_running(true);
        for index in 0..=PUBLISHER_CAPACITY {
            publisher.on_instance_roster(InstanceRosterSnapshot {
                location: index.to_string(),
                ..Default::default()
            });
        }
        publisher.on_game_running(false);

        let mut transitions = Vec::new();
        while transitions.len() < 2 {
            if let Some(LocalApiInput::GameRunning { running, .. }) = receiver.recv().await {
                transitions.push(running);
            }
        }
        assert_eq!(transitions, vec![true, false]);
    }

    #[tokio::test]
    async fn roster_from_a_previous_lifecycle_is_not_delivered_after_restart() {
        let (publisher, mut receiver) = local_api_publisher_channel();
        publisher.on_game_running(true);
        assert!(matches!(
            receiver.recv().await,
            Some(LocalApiInput::GameRunning { running: true, .. })
        ));
        publisher.on_instance_roster(InstanceRosterSnapshot {
            location: "wrld_old:1".into(),
            ..Default::default()
        });
        publisher.on_game_running(false);
        publisher.on_game_running(true);

        assert!(matches!(
            receiver.recv().await,
            Some(LocalApiInput::GameRunning {
                lifecycle_epoch: 2,
                running: false,
            })
        ));
        let Some(LocalApiInput::GameRunning {
            lifecycle_epoch,
            running: true,
        }) = receiver.recv().await
        else {
            panic!("expected the restarted lifecycle state");
        };
        assert_eq!(lifecycle_epoch, 3);

        publisher.on_instance_roster(InstanceRosterSnapshot {
            location: "wrld_new:2".into(),
            ..Default::default()
        });
        let Some(LocalApiInput::Roster {
            lifecycle_epoch: roster_epoch,
            snapshot,
        }) = receiver.recv().await
        else {
            panic!("expected a current roster");
        };
        assert_eq!(roster_epoch, lifecycle_epoch);
        assert_eq!(snapshot.location, "wrld_new:2");
    }
}
