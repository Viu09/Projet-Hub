#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

static ROOM_STATE: OnceLock<Mutex<HashMap<String, RoomInfo>>> = OnceLock::new();
static NEXT_ROOM_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomsResponse {
    pub rooms: Vec<RoomInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_id: String,
    pub name: String,
    pub server_addr: String,
    pub region: String,
    pub players: u8,
    pub max_players: u8,
    pub is_private: bool,
    pub status: RoomStatus,
    pub ping_ms: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomStatus {
    Waiting,
    Starting,
    Running,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub region: String,
    pub max_players: u8,
    pub is_private: bool,
    pub access_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
    pub server_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRoomRequest {
    pub room_id: String,
    pub player_name: String,
    pub access_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRoomResponse {
    pub token: String,
    pub server_addr: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub server_addr: String,
    pub room_id: String,
    pub players: u8,
    pub max_players: u8,
    pub status: RoomStatus,
    pub region: String,
    pub updated_at: u64,
}

pub fn upsert_room(room: RoomInfo) {
    let map = ROOM_STATE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = map.lock() {
        guard.insert(room.room_id.clone(), room);
    }
}

pub fn create_room(req: CreateRoomRequest) -> RoomInfo {
    let id = NEXT_ROOM_ID.fetch_add(1, Ordering::Relaxed);
    let room_id = format!("LOCAL-{}", id);
    let room = RoomInfo {
        room_id: room_id.clone(),
        name: if req.name.trim().is_empty() { "Local Room".to_owned() } else { req.name },
        server_addr: "ws://127.0.0.1:9001".to_owned(),
        region: req.region,
        players: 0,
        max_players: req.max_players.max(2).min(8),
        is_private: req.is_private,
        status: RoomStatus::Waiting,
        ping_ms: Some(1),
    };
    upsert_room(room.clone());
    room
}

pub fn delete_room(room_id: &str) -> bool {
    let map = ROOM_STATE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = map.lock() {
        return guard.remove(room_id).is_some();
    }
    false
}

pub fn list_rooms() -> Vec<RoomInfo> {
    let map = ROOM_STATE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = map.lock() {
        return guard.values().cloned().collect();
    }
    Vec::new()
}

pub fn get_room(room_id: &str) -> Option<RoomInfo> {
    let map = ROOM_STATE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = map.lock() {
        return guard.get(room_id).cloned();
    }
    None
}
