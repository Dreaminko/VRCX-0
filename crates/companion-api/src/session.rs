use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use tokio::sync::broadcast;

use crate::state::{wire_member, wire_room, RoomChange};
use crate::transport::{now_iso, CompanionApiRouterState, ServerEvent};
use crate::wire::{ClientMessage, ServerMessage, HEARTBEAT_SECONDS, PROTOCOL_VERSION};

const MAX_ACTIVE_CONNECTIONS: u32 = 8;

pub(crate) async fn run_session(mut socket: WebSocket, state: CompanionApiRouterState) {
    let Some(_guard) = ActiveConnectionGuard::try_new(Arc::clone(&state.active_connections)) else {
        let _ = socket
            .send(Message::Close(Some(CloseFrame {
                code: 1008,
                reason: "too many connections".into(),
            })))
            .await;
        return;
    };

    let (mut receiver, initial_snapshot) = state.hub.subscribe();
    let mut room_revision = initial_snapshot.revision;
    let mut seq = 0_u64;
    if !send_message(
        &mut socket,
        &ServerMessage::Hello {
            seq,
            protocol: PROTOCOL_VERSION,
            app: "vrcx-0".into(),
            app_version: state.hub.app_version().into(),
            scopes: vec!["room".into()],
            heartbeat_sec: HEARTBEAT_SECONDS,
        },
    )
    .await
    {
        return;
    }
    seq = seq.saturating_add(1);
    if !send_snapshot(&mut socket, seq, initial_snapshot.room.as_ref(), now_iso()).await {
        return;
    }

    let start = tokio::time::Instant::now() + Duration::from_secs(HEARTBEAT_SECONDS);
    let mut heartbeat = tokio::time::interval_at(start, Duration::from_secs(HEARTBEAT_SECONDS));
    loop {
        tokio::select! {
            _ = state.session_cancel.cancelled() => {
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(text.as_str()) {
                            Ok(ClientMessage::Resync) => {
                                seq = seq.saturating_add(1);
                                let Some(revision) = send_latest_snapshot(
                                    &mut socket,
                                    seq,
                                    &state,
                                ).await else { break; };
                                room_revision = revision;
                            }
                            Err(error) => {
                                tracing::debug!(
                                    error = %error,
                                    "ignored invalid Companion API client frame"
                                );
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        tracing::debug!(error = %error, "Companion API WebSocket receive failed");
                        break;
                    }
                }
            }
            event = receiver.recv() => {
                match event {
                    Ok(ServerEvent::Snapshot { revision, room, at }) => {
                        if revision <= room_revision {
                            continue;
                        }
                        seq = seq.saturating_add(1);
                        if revision != room_revision.saturating_add(1) {
                            let Some(revision) = send_latest_snapshot(
                                &mut socket,
                                seq,
                                &state,
                            ).await else { break; };
                            room_revision = revision;
                            continue;
                        }
                        if !send_snapshot(&mut socket, seq, room.as_ref(), at).await {
                            break;
                        }
                        room_revision = revision;
                    }
                    Ok(ServerEvent::Changes { revision, changes, at }) => {
                        if revision <= room_revision {
                            continue;
                        }
                        if revision != room_revision.saturating_add(1) {
                            seq = seq.saturating_add(1);
                            let Some(revision) = send_latest_snapshot(
                                &mut socket,
                                seq,
                                &state,
                            ).await else { break; };
                            room_revision = revision;
                            continue;
                        }
                        for change in changes {
                            seq = seq.saturating_add(1);
                            let message = message_for_change(change, seq, &at);
                            if !send_message(&mut socket, &message).await {
                                return;
                            }
                        }
                        room_revision = revision;
                    }
                    Ok(ServerEvent::Bye(reason)) => {
                        let _ = send_message(&mut socket, &ServerMessage::Bye { reason }).await;
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        seq = seq.saturating_add(1);
                        let Some(revision) = send_latest_snapshot(
                            &mut socket,
                            seq,
                            &state,
                        ).await else { break; };
                        room_revision = revision;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = heartbeat.tick() => {
                seq = seq.saturating_add(1);
                if !send_message(
                    &mut socket,
                    &ServerMessage::Ping { seq, at: now_iso() },
                ).await {
                    break;
                }
            }
        }
    }
}

fn message_for_change(change: RoomChange, seq: u64, at: &str) -> ServerMessage {
    match change {
        RoomChange::Snapshot(room) => ServerMessage::RoomSnapshot {
            seq,
            at: at.into(),
            room: Some(wire_room(&room)),
        },
        RoomChange::Joined(members) => ServerMessage::RoomJoined {
            seq,
            at: at.into(),
            members: members.iter().map(wire_member).collect(),
        },
        RoomChange::Left(user_ids) => ServerMessage::RoomLeft {
            seq,
            at: at.into(),
            user_ids,
        },
    }
}

async fn send_snapshot(
    socket: &mut WebSocket,
    seq: u64,
    room: Option<&crate::state::RoomState>,
    at: String,
) -> bool {
    send_message(
        socket,
        &ServerMessage::RoomSnapshot {
            seq,
            at,
            room: room.map(wire_room),
        },
    )
    .await
}

async fn send_latest_snapshot(
    socket: &mut WebSocket,
    seq: u64,
    state: &CompanionApiRouterState,
) -> Option<u64> {
    let snapshot = state.hub.snapshot();
    send_snapshot(socket, seq, snapshot.room.as_ref(), now_iso())
        .await
        .then_some(snapshot.revision)
}

async fn send_message(socket: &mut WebSocket, message: &ServerMessage) -> bool {
    let Ok(payload) = serde_json::to_string(message) else {
        return false;
    };
    socket.send(Message::Text(payload.into())).await.is_ok()
}

struct ActiveConnectionGuard {
    active_connections: Arc<AtomicU32>,
}

impl ActiveConnectionGuard {
    fn try_new(active_connections: Arc<AtomicU32>) -> Option<Self> {
        let mut current = active_connections.load(Ordering::Relaxed);
        loop {
            if current >= MAX_ACTIVE_CONNECTIONS {
                return None;
            }
            match active_connections.compare_exchange_weak(
                current,
                current.saturating_add(1),
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(Self { active_connections }),
                Err(observed) => current = observed,
            }
        }
    }
}

impl Drop for ActiveConnectionGuard {
    fn drop(&mut self) {
        self.active_connections.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_connection_limit_is_atomic_and_released_on_drop() {
        let count = Arc::new(AtomicU32::new(0));
        let guards = (0..MAX_ACTIVE_CONNECTIONS)
            .map(|_| ActiveConnectionGuard::try_new(Arc::clone(&count)).unwrap())
            .collect::<Vec<_>>();
        assert!(ActiveConnectionGuard::try_new(Arc::clone(&count)).is_none());
        assert_eq!(count.load(Ordering::Relaxed), MAX_ACTIVE_CONNECTIONS);
        drop(guards);
        assert_eq!(count.load(Ordering::Relaxed), 0);
    }
}
