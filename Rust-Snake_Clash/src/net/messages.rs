use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientEnvelope {
    pub v: u8,
    #[serde(flatten)]
    pub msg: ClientMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEnvelope {
    pub v: u8,
    #[serde(flatten)]
    pub msg: ServerMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "data", rename_all = "snake_case")]
pub enum ClientMessage {
    JoinReq {
        room_id: String,
        name: String,
        device: String,
        client_time: f32,
    },
    Input {
        seq: u32,
        tick: u32,
        dir: Vec2f,
        boost: bool,
        client_time: f32,
        last_snapshot_ack: Option<u32>,
    },
    Ping {
        client_time: f32,
    },
    Leave,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "data", rename_all = "snake_case")]
pub enum ServerMessage {
    JoinOk {
        player_id: u32,
        tick_rate: u16,
        server_tick: u32,
        arena: ArenaInfo,
    },
    Snapshot {
        server_tick: u32,
        players: Vec<PlayerState>,
        pellets: Vec<Vec2f>,
        tokens: Vec<TokenState>,
        events: Vec<Event>,
        time_left: f32,
        countdown_left: f32,
    },
    SnapshotDelta {
        server_tick: u32,
        base_tick: u32,
        players: Vec<PlayerDelta>,
        pellets: Vec<Vec2f>,
        tokens: Vec<TokenState>,
        events: Vec<Event>,
        time_left: f32,
        countdown_left: f32,
    },
    Pong {
        server_time: f32,
        client_time: f32,
    },
    PlayerLeft {
        id: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaInfo {
    pub radius: f32,
    pub seed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: u32,
    pub alive: bool,
    pub head: Vec2f,
    pub dir: Vec2f,
    pub radius: f32,
    pub score: i32,
    pub boost: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDelta {
    pub id: u32,
    pub field_mask: u16,
    pub alive: Option<bool>,
    pub head: Option<Vec2f>,
    pub dir: Option<Vec2f>,
    pub radius: Option<f32>,
    pub score: Option<i32>,
    pub boost: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenState {
    pub id: u32,
    pub kind: String,
    pub pos: Vec2f,
    pub ttl: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    pub id: u32,
}
