use std::collections::HashMap;

use std::collections::VecDeque;

use crate::config::{BASE_SNAKE_LENGTH, SCORE_PER_SEGMENT, SNAKE_SPACING_MULT};
use crate::net::messages::{Event, PlayerDelta, PlayerState, ServerMessage, TokenState, Vec2f};

#[derive(Default)]
pub struct SnapshotBuffer {
    pub snapshots: Vec<ServerMessage>,
    pub last_snapshot_tick: u32,
    pub players: HashMap<u32, PlayerState>,
    pub pellets: Vec<Vec2f>,
    pub tokens: Vec<TokenState>,
    pub events: Vec<Event>,
    pub time_left: f32,
    pub countdown_left: f32,
    trails: HashMap<u32, RenderTrail>,
}

impl SnapshotBuffer {
    pub fn push(&mut self, msg: ServerMessage) {
        self.apply_message(&msg);
        if let ServerMessage::Snapshot { server_tick, .. } = &msg {
            self.last_snapshot_tick = *server_tick;
        }
        if let ServerMessage::SnapshotDelta { server_tick, .. } = &msg {
            self.last_snapshot_tick = *server_tick;
        }
        self.snapshots.push(msg);
        if self.snapshots.len() > 4 {
            self.snapshots.remove(0);
        }
    }

    pub fn apply_message(&mut self, msg: &ServerMessage) {
        match msg {
            ServerMessage::Snapshot {
                players,
                pellets,
                tokens,
                events,
                time_left,
                countdown_left,
                ..
            } => {
                self.players.clear();
                for p in players {
                    self.players.insert(p.id, p.clone());
                }
                self.pellets = pellets.clone();
                self.tokens = tokens.clone();
                self.events = events.clone();
                self.time_left = *time_left;
                self.countdown_left = *countdown_left;
                self.update_trails();
            }
            ServerMessage::SnapshotDelta {
                players,
                pellets,
                tokens,
                events,
                time_left,
                countdown_left,
                ..
            } => {
                for delta in players {
                    apply_delta(&mut self.players, delta);
                }
                self.pellets = pellets.clone();
                self.tokens = tokens.clone();
                self.events.extend(events.iter().cloned());
                self.time_left = *time_left;
                self.countdown_left = *countdown_left;
                self.update_trails();
            }
            _ => {}
        }
    }

    pub fn players_vec(&self) -> Vec<PlayerState> {
        self.players.values().cloned().collect()
    }

    pub fn pellets_vec(&self) -> Vec<Vec2f> {
        self.pellets.clone()
    }

    pub fn tokens_vec(&self) -> Vec<TokenState> {
        self.tokens.clone()
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        std::mem::swap(&mut out, &mut self.events);
        out
    }

    pub fn trail_for(&self, player_id: u32) -> Vec<Vec2f> {
        self.trails
            .get(&player_id)
            .map(|t| t.points.iter().copied().collect())
            .unwrap_or_default()
    }

    fn update_trails(&mut self) {
        for player in self.players.values() {
            let entry = self.trails.entry(player.id).or_default();
            entry.push(player.head, player.radius, player.score);
        }
        let live_ids: Vec<u32> = self.players.keys().copied().collect();
        self.trails.retain(|id, _| live_ids.contains(id));
    }
}

fn apply_delta(players: &mut HashMap<u32, PlayerState>, delta: &PlayerDelta) {
    let entry = players.entry(delta.id).or_insert(PlayerState {
        id: delta.id,
        alive: true,
        head: crate::net::messages::Vec2f { x: 0.0, y: 0.0 },
        dir: crate::net::messages::Vec2f { x: 1.0, y: 0.0 },
        radius: 18.0,
        score: 0,
        boost: 100.0,
    });

    if let Some(v) = delta.alive {
        entry.alive = v;
    }
    if let Some(v) = delta.head {
        entry.head = v;
    }
    if let Some(v) = delta.dir {
        entry.dir = v;
    }
    if let Some(v) = delta.radius {
        entry.radius = v;
    }
    if let Some(v) = delta.score {
        entry.score = v;
    }
    if let Some(v) = delta.boost {
        entry.boost = v;
    }
}

#[derive(Default)]
struct RenderTrail {
    points: VecDeque<Vec2f>,
}

impl RenderTrail {
    fn push(&mut self, head: Vec2f, radius: f32, score: i32) {
        self.points.push_front(head);
        let extra = (score / SCORE_PER_SEGMENT).max(0) as usize;
        let target_length = (BASE_SNAKE_LENGTH + extra).clamp(BASE_SNAKE_LENGTH, 900);
        let spacing = (radius * SNAKE_SPACING_MULT).max(5.2);
        let max_points = (target_length as f32 * (spacing / 10.0)).ceil() as usize;
        let max_points = max_points.clamp(12, 260);
        while self.points.len() > max_points {
            self.points.pop_back();
        }
    }
}
