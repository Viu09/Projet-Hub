use crate::config::ARENA_RADIUS;
use std::collections::HashMap;

use macroquad::prelude::*;

use crate::net::messages::{
    ArenaInfo, ClientMessage, PlayerDelta, PlayerState, ServerMessage, TokenState, Vec2f,
};
use crate::game::food::TokenKind;
use crate::state::room::InputState;
use crate::master::state::{RoomInfo, RoomStatus};
use crate::net::session::OutboundMessage;
use crate::state::room::Room;

// Rooms / matchmaking (minimal, functional skeleton).
pub struct Lobby {
    rooms: HashMap<String, Room>,
    next_room_id: u32,
    last_snapshot_ack: HashMap<u64, u32>,
    last_snapshot: HashMap<u64, SnapshotCache>,
    session_rooms: HashMap<u64, String>,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            next_room_id: 1,
            last_snapshot_ack: HashMap::new(),
            last_snapshot: HashMap::new(),
            session_rooms: HashMap::new(),
        }
    }

    pub fn handle_message(&mut self, session_id: u64, msg: ClientMessage) -> Vec<OutboundMessage> {
        match msg {
            ClientMessage::JoinReq { room_id, .. } => {
                let room = self
                    .rooms
                    .entry(room_id.clone())
                    .or_insert_with(|| {
                        let id = self.next_room_id;
                        self.next_room_id = self.next_room_id.saturating_add(1);
                        Room::new(id, 20)
                    });
                let player_id = room.add_player(session_id);
                if player_id == 0 {
                    return Vec::new();
                }
                self.session_rooms.insert(session_id, room_id);
                self.last_snapshot_ack.insert(session_id, 0);
                self.last_snapshot.insert(
                    session_id,
                    SnapshotCache {
                        tick: 0,
                        players: Vec::new(),
                    },
                );
                vec![OutboundMessage {
                    session_id,
                    message: ServerMessage::JoinOk {
                        player_id,
                        tick_rate: room.tick_rate,
                        server_tick: 0,
                        arena: ArenaInfo {
                            radius: ARENA_RADIUS,
                            seed: 42,
                        },
                    },
                }]
            }
            ClientMessage::Ping { client_time } => vec![OutboundMessage {
                session_id,
                message: ServerMessage::Pong {
                    server_time: 0.0,
                    client_time,
                },
            }],
            ClientMessage::Leave => self.handle_disconnect(session_id),
            ClientMessage::Input {
                dir,
                boost,
                last_snapshot_ack,
                ..
            } => {
                if let Some(ack) = last_snapshot_ack {
                    self.last_snapshot_ack.insert(session_id, ack);
                }
                if let Some(room_id) = self.session_rooms.get(&session_id).cloned() {
                    if let Some(room) = self.rooms.get_mut(&room_id) {
                        room.set_input(
                            session_id,
                            InputState {
                                dir: vec2(dir.x, dir.y),
                                boost,
                            },
                        );
                    }
                }
                Vec::new()
            }
        }
    }

    pub fn handle_disconnect(&mut self, session_id: u64) -> Vec<OutboundMessage> {
        let mut outbound = Vec::new();
        let room_id = self.session_rooms.remove(&session_id);
        if let Some(room_id) = room_id {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                if let Some(player_id) = room.remove_player(session_id) {
                    self.last_snapshot_ack.remove(&session_id);
                    self.last_snapshot.remove(&session_id);
                    for other in room.session_ids() {
                        outbound.push(OutboundMessage {
                            session_id: other,
                            message: ServerMessage::PlayerLeft { id: player_id },
                        });
                    }
                }
            }
        }
        outbound
    }

    pub fn tick(&mut self) -> Vec<OutboundMessage> {
        let mut outbound = Vec::new();

        let mut room_sessions: HashMap<String, Vec<u64>> = HashMap::new();
        for (session_id, room_id) in &self.session_rooms {
            room_sessions
                .entry(room_id.clone())
                .or_default()
                .push(*session_id);
        }

        for (room_id, sessions) in room_sessions {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                let _ = room.id;
                room.step();

                let players = room.player_states();
                let server_tick = room.world.server_tick;
                let pellets = build_pellets(room);
                let tokens = build_tokens(room);
                let events = room.take_events();
                let time_left = room.time_left();
                let countdown_left = room.countdown_left();

                for session_id in sessions {
                    let last_ack = self.last_snapshot_ack.get(&session_id).copied().unwrap_or(0);
                    if let Some(cache) = self.last_snapshot.get(&session_id) {
                        if last_ack == cache.tick {
                            let delta_players = build_player_deltas(&cache.players, &players);
                            let message = ServerMessage::SnapshotDelta {
                                server_tick,
                                base_tick: cache.tick,
                                players: delta_players,
                                pellets: pellets.clone(),
                                tokens: tokens.clone(),
                                events: events.clone(),
                                time_left,
                                countdown_left,
                            };
                            outbound.push(OutboundMessage { session_id, message });
                            self.last_snapshot.insert(
                                session_id,
                                SnapshotCache {
                                    tick: server_tick,
                                    players: players.clone(),
                                },
                            );
                            continue;
                        }
                    }

                    let message = ServerMessage::Snapshot {
                        server_tick,
                        players: players.clone(),
                        pellets: pellets.clone(),
                        tokens: tokens.clone(),
                        events: events.clone(),
                        time_left,
                        countdown_left,
                    };
                    outbound.push(OutboundMessage { session_id, message });
                    self.last_snapshot.insert(
                        session_id,
                        SnapshotCache {
                            tick: server_tick,
                            players: players.clone(),
                        },
                    );
                }
            }
        }

        publish_room_stats(&self.session_rooms);

        outbound
    }
}

#[derive(Clone)]
struct SnapshotCache {
    tick: u32,
    players: Vec<PlayerState>,
}

const MASK_ALIVE: u16 = 1 << 0;
const MASK_HEAD: u16 = 1 << 1;
const MASK_DIR: u16 = 1 << 2;
const MASK_RADIUS: u16 = 1 << 3;
const MASK_SCORE: u16 = 1 << 4;
const MASK_BOOST: u16 = 1 << 5;

fn build_player_deltas(prev: &[PlayerState], next: &[PlayerState]) -> Vec<PlayerDelta> {
    let mut prev_map: HashMap<u32, &PlayerState> = HashMap::new();
    for p in prev {
        prev_map.insert(p.id, p);
    }

    let mut deltas = Vec::new();
    for p in next {
        let mut mask = 0u16;
        let mut delta = PlayerDelta {
            id: p.id,
            field_mask: 0,
            alive: None,
            head: None,
            dir: None,
            radius: None,
            score: None,
            boost: None,
        };

        if let Some(prev_p) = prev_map.get(&p.id) {
            if prev_p.alive != p.alive {
                mask |= MASK_ALIVE;
                delta.alive = Some(p.alive);
            }
            if prev_p.head != p.head {
                mask |= MASK_HEAD;
                delta.head = Some(p.head);
            }
            if prev_p.dir != p.dir {
                mask |= MASK_DIR;
                delta.dir = Some(p.dir);
            }
            if (prev_p.radius - p.radius).abs() > f32::EPSILON {
                mask |= MASK_RADIUS;
                delta.radius = Some(p.radius);
            }
            if prev_p.score != p.score {
                mask |= MASK_SCORE;
                delta.score = Some(p.score);
            }
            if (prev_p.boost - p.boost).abs() > f32::EPSILON {
                mask |= MASK_BOOST;
                delta.boost = Some(p.boost);
            }
        } else {
            mask |= MASK_ALIVE | MASK_HEAD | MASK_DIR | MASK_RADIUS | MASK_SCORE | MASK_BOOST;
            delta.alive = Some(p.alive);
            delta.head = Some(p.head);
            delta.dir = Some(p.dir);
            delta.radius = Some(p.radius);
            delta.score = Some(p.score);
            delta.boost = Some(p.boost);
        }

        if mask != 0 {
            delta.field_mask = mask;
            deltas.push(delta);
        }
    }

    deltas
}

fn token_kind_to_string(kind: TokenKind) -> String {
    match kind {
        TokenKind::Magnet => "magnet".to_owned(),
        TokenKind::SpeedUp => "speed".to_owned(),
        TokenKind::TimeAdd => "time".to_owned(),
    }
}

fn build_pellets(room: &Room) -> Vec<Vec2f> {
    room.pellets
        .positions()
        .into_iter()
        .map(|p| Vec2f { x: p.x, y: p.y })
        .collect()
}

fn build_tokens(room: &Room) -> Vec<TokenState> {
    room.tokens
        .items()
        .iter()
        .enumerate()
        .map(|(idx, t)| TokenState {
            id: idx as u32,
            kind: token_kind_to_string(t.kind),
            pos: Vec2f { x: t.pos.x, y: t.pos.y },
            ttl: 0.0,
        })
        .collect()
}

fn publish_room_stats(session_rooms: &HashMap<u64, String>) {
    let mut counts: HashMap<String, u8> = HashMap::new();
    for room_id in session_rooms.values() {
        let entry = counts.entry(room_id.clone()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    for (room_id, players) in counts {
        let existing = crate::master::state::get_room(&room_id);
        let base = existing.unwrap_or(RoomInfo {
            room_id: room_id.clone(),
            name: "Local Room".to_owned(),
            server_addr: "ws://127.0.0.1:9001".to_owned(),
            region: "LOCAL".to_owned(),
            players: 0,
            max_players: 4,
            is_private: false,
            status: RoomStatus::Waiting,
            ping_ms: Some(1),
        });

        crate::master::state::upsert_room(RoomInfo {
            players,
            status: if players > 0 { RoomStatus::Running } else { RoomStatus::Waiting },
            ..base
        });
    }
}
