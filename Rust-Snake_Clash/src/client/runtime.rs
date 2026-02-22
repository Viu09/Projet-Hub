use std::sync::{Arc, Mutex, OnceLock};

use crate::client::net::ClientRuntime;
use crate::client::state::SnapshotBuffer;
use crate::net::messages::{ClientMessage, Vec2f};

static CLIENT_HANDLE: OnceLock<Arc<ClientHandle>> = OnceLock::new();

pub struct ClientHandle {
    runtime: ClientRuntime,
    snapshots: Mutex<SnapshotBuffer>,
    seq: Mutex<u32>,
    player_id: Mutex<Option<u32>>,
}

impl ClientHandle {
    fn new(runtime: ClientRuntime) -> Self {
        Self {
            runtime,
            snapshots: Mutex::new(SnapshotBuffer::default()),
            seq: Mutex::new(0),
            player_id: Mutex::new(None),
        }
    }
}

pub fn init(server_url: String) {
    if CLIENT_HANDLE.get().is_some() {
        return;
    }
    let runtime = ClientRuntime::connect(server_url);
    let handle = ClientHandle::new(runtime);
    let _ = CLIENT_HANDLE.set(Arc::new(handle));
}

pub fn is_ready() -> bool {
    CLIENT_HANDLE.get().is_some()
}

pub fn poll() {
    if let Some(handle) = CLIENT_HANDLE.get() {
        while let Some(msg) = handle.runtime.try_recv() {
            if let crate::net::messages::ServerMessage::JoinOk { player_id, .. } = msg {
                if let Ok(mut guard) = handle.player_id.lock() {
                    *guard = Some(player_id);
                }
            }
            if let Ok(mut guard) = handle.snapshots.lock() {
                guard.push(msg);
            }
        }
    }
}

pub fn send_input(dir: Vec2f, boost: bool) {
    if let Some(handle) = CLIENT_HANDLE.get() {
        let ack = handle
            .snapshots
            .lock()
            .map(|guard| guard.last_snapshot_tick)
            .unwrap_or(0);
        let mut seq_guard = handle.seq.lock().unwrap();
        *seq_guard = seq_guard.saturating_add(1);
        let seq = *seq_guard;
        handle.runtime.send(ClientMessage::Input {
            seq,
            tick: ack,
            dir,
            boost,
            client_time: 0.0,
            last_snapshot_ack: Some(ack),
        });
    }
}

pub fn send_join(room_id: String, name: String, device: String) {
    if let Some(handle) = CLIENT_HANDLE.get() {
        handle.runtime.send(ClientMessage::JoinReq {
            room_id,
            name,
            device,
            client_time: 0.0,
        });
    }
}

pub fn latest_players() -> Vec<crate::net::messages::PlayerState> {
    if let Some(handle) = CLIENT_HANDLE.get() {
        if let Ok(guard) = handle.snapshots.lock() {
            return guard.players_vec();
        }
    }
    Vec::new()
}

pub fn latest_time_left() -> f32 {
    if let Some(handle) = CLIENT_HANDLE.get() {
        if let Ok(guard) = handle.snapshots.lock() {
            return guard.time_left;
        }
    }
    0.0
}

pub fn latest_countdown_left() -> f32 {
    if let Some(handle) = CLIENT_HANDLE.get() {
        if let Ok(guard) = handle.snapshots.lock() {
            return guard.countdown_left;
        }
    }
    0.0
}

pub fn latest_pellets() -> Vec<crate::net::messages::Vec2f> {
    if let Some(handle) = CLIENT_HANDLE.get() {
        if let Ok(guard) = handle.snapshots.lock() {
            return guard.pellets_vec();
        }
    }
    Vec::new()
}

pub fn latest_tokens() -> Vec<crate::net::messages::TokenState> {
    if let Some(handle) = CLIENT_HANDLE.get() {
        if let Ok(guard) = handle.snapshots.lock() {
            return guard.tokens_vec();
        }
    }
    Vec::new()
}

pub fn drain_events() -> Vec<crate::net::messages::Event> {
    if let Some(handle) = CLIENT_HANDLE.get() {
        if let Ok(mut guard) = handle.snapshots.lock() {
            return guard.take_events();
        }
    }
    Vec::new()
}

pub fn trail_for(player_id: u32) -> Vec<crate::net::messages::Vec2f> {
    if let Some(handle) = CLIENT_HANDLE.get() {
        if let Ok(guard) = handle.snapshots.lock() {
            return guard.trail_for(player_id);
        }
    }
    Vec::new()
}

pub fn local_player_id() -> Option<u32> {
    if let Some(handle) = CLIENT_HANDLE.get() {
        if let Ok(guard) = handle.player_id.lock() {
            return *guard;
        }
    }
    None
}
