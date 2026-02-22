#![allow(dead_code)]

use serde::{Deserialize, Serialize};

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
    pub status: String,
    pub ping_ms: Option<u16>,
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

pub fn fetch_rooms() -> Vec<RoomInfo> {
    let url = "http://127.0.0.1:9100/rooms";
    let response = ureq::get(url).call();
    if let Ok(resp) = response {
        if let Ok(body) = resp.into_json::<RoomsResponse>() {
            return body.rooms;
        }
    }
    vec![RoomInfo {
        room_id: "DEV-ROOM".to_owned(),
        name: "Local Room".to_owned(),
        server_addr: "ws://127.0.0.1:9001".to_owned(),
        region: "LOCAL".to_owned(),
        players: 0,
        max_players: 4,
        is_private: false,
        status: "waiting".to_owned(),
        ping_ms: Some(1),
    }]
}

pub fn join_room(room_id: &str, _player_name: &str, _access_code: Option<&str>) -> Option<String> {
    let rooms = fetch_rooms();
    rooms
        .into_iter()
        .find(|r| r.room_id == room_id)
        .map(|r| r.server_addr)
}

pub fn create_room(name: &str, max_players: u8) -> Option<RoomInfo> {
    let url = "http://127.0.0.1:9100/rooms";
    let req = CreateRoomRequest {
        name: name.to_owned(),
        region: "LOCAL".to_owned(),
        max_players,
        is_private: false,
        access_code: None,
    };
    let response = ureq::post(url).send_json(req);
    if let Ok(resp) = response {
        if let Ok(body) = resp.into_json::<CreateRoomResponse>() {
            return Some(RoomInfo {
                room_id: body.room_id,
                name: name.to_owned(),
                server_addr: body.server_addr,
                region: "LOCAL".to_owned(),
                players: 0,
                max_players,
                is_private: false,
                status: "waiting".to_owned(),
                ping_ms: Some(1),
            });
        }
    }
    None
}

pub fn delete_room(room_id: &str) -> bool {
    let url = format!("http://127.0.0.1:9100/rooms/{}", room_id);
    ureq::delete(&url).call().is_ok()
}
