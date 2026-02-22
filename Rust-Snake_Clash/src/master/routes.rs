#![allow(dead_code)]

use axum::{routing::{delete, get, post}, extract::Path, Json, Router};

use crate::master::state::{
    CreateRoomRequest, CreateRoomResponse, JoinRoomRequest, JoinRoomResponse, RoomsResponse, RoomInfo, RoomStatus,
    HeartbeatRequest,
};
use crate::master::state;

pub fn router() -> Router {
    Router::new()
        .route("/rooms", get(list_rooms))
        .route("/rooms", post(create_room))
    .route("/rooms/:room_id", delete(delete_room))
        .route("/rooms/join", post(join_room))
        .route("/rooms/heartbeat", post(heartbeat))
}

async fn list_rooms() -> Json<RoomsResponse> {
    Json(RoomsResponse { rooms: state::list_rooms() })
}

async fn create_room(Json(req): Json<CreateRoomRequest>) -> Json<CreateRoomResponse> {
    let room = state::create_room(req);
    Json(CreateRoomResponse {
        room_id: room.room_id,
        server_addr: room.server_addr,
    })
}

async fn delete_room(Path(room_id): Path<String>) -> Json<()> {
    let _ = state::delete_room(&room_id);
    Json(())
}

async fn join_room(Json(_req): Json<JoinRoomRequest>) -> Json<JoinRoomResponse> {
    Json(JoinRoomResponse {
        token: "dev-token".to_owned(),
        server_addr: "ws://127.0.0.1:9001".to_owned(),
        expires_at: 0,
    })
}

async fn heartbeat(Json(_req): Json<HeartbeatRequest>) -> Json<()> {
    Json(())
}

#[allow(dead_code)]
fn demo_room() -> RoomInfo {
    RoomInfo {
        room_id: "EU-1A2B".to_owned(),
        name: "Public #12".to_owned(),
        server_addr: "wss://game-01.example.com:9001".to_owned(),
        region: "EU".to_owned(),
        players: 2,
        max_players: 4,
        is_private: false,
        status: RoomStatus::Waiting,
        ping_ms: Some(32),
    }
}
