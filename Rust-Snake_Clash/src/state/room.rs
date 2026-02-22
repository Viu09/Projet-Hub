use std::collections::{HashMap, HashSet};

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::game::sim::WorldState;
use crate::game::food::{Pellets, TokenKind, Tokens};
use crate::game::snake_sim::SnakeSim;
use crate::config::{
    ARENA_RADIUS, BASE_SNAKE_LENGTH, BASE_SNAKE_RADIUS, BASE_SPEED, BOOST_ENERGY_DRAIN_PER_SEC,
    BOOST_ENERGY_MAX, BOOST_ENERGY_REGEN_PER_SEC, BOOST_SPEED_MULT, MAGNET_ATTRACT_MAX_PER_FRAME,
    MAGNET_ATTRACT_RADIUS, MAGNET_ATTRACT_SPEED, MAGNET_PICKUP_BONUS_PX, PELLET_BUCKET_SIZE,
    PELLET_EAT_MAX_PER_FRAME, PELLET_RADIUS, PELLET_TARGET_COUNT, SCORE_PER_SEGMENT,
    SNAKE_RADIUS_GROWTH_EXP, SNAKE_RADIUS_SCORE_HALF, SNAKE_SPACING_MAX, SNAKE_SPACING_MULT,
    SMALL_SNAKE_SPEED_MULT, SPEEDUP_MULT, TOKEN_DURATION_SEC, TOKEN_TARGET_COUNT,
    TOKEN_TIME_ADD_SEC, MAX_SNAKE_RADIUS, MATCH_DURATION_SEC, MATCH_START_COUNTDOWN_SEC,
    CORPSE_DROP_MAX_PELLETS, CORPSE_DROP_SPREAD_PX,
};
use crate::net::messages::{Event, PlayerState, Vec2f};

pub struct Room {
    #[allow(dead_code)]
    pub id: u32,
    pub tick_rate: u16,
    players: HashMap<u64, PlayerEntity>,
    next_player_id: u32,
    inputs: HashMap<u64, InputState>,
    pub world: WorldState,
    pub pellets: Pellets,
    pub tokens: Tokens,
    events: Vec<Event>,
    time_left: f32,
    countdown_left: f32,
    finished: bool,
}

impl Room {
    pub fn new(id: u32, tick_rate: u16) -> Self {
        Self {
            id,
            tick_rate,
            players: HashMap::new(),
            next_player_id: 1,
            inputs: HashMap::new(),
            world: WorldState::default(),
            pellets: Pellets::new(PELLET_BUCKET_SIZE, ARENA_RADIUS),
            tokens: Tokens::new(ARENA_RADIUS, TOKEN_TARGET_COUNT),
            events: Vec::new(),
            time_left: MATCH_DURATION_SEC,
            countdown_left: MATCH_START_COUNTDOWN_SEC,
            finished: false,
        }
    }

    pub fn add_player(&mut self, session_id: u64) -> u32 {
        if self.players.len() >= 4 {
            return 0;
        }
        let player_id = self.next_player_id;
        self.next_player_id = self.next_player_id.saturating_add(1);
        let pos = random_pos_in_disk(ARENA_RADIUS * 0.6);
        let dir = random_unit_dir();
        self.players.insert(
            session_id,
            PlayerEntity {
                id: player_id,
                alive: true,
                snake: SnakeSim::new_at(pos, dir),
                score: 0,
                boost_energy: BOOST_ENERGY_MAX,
                magnet_left: 0.0,
                speedup_left: 0.0,
            },
        );
        self.inputs.insert(session_id, InputState::default());
        player_id
    }

    pub fn remove_player(&mut self, session_id: u64) -> Option<u32> {
        self.inputs.remove(&session_id);
        self.players.remove(&session_id).map(|p| p.id)
    }

    pub fn session_ids(&self) -> Vec<u64> {
        self.players.keys().copied().collect()
    }

    pub fn set_input(&mut self, session_id: u64, input: InputState) {
        if self.players.contains_key(&session_id) {
            self.inputs.insert(session_id, input);
        }
    }

    pub fn step(&mut self) {
        let dt = 1.0 / (self.tick_rate as f32).max(1.0);
        self.world.step();
        self.events.clear();

        if self.countdown_left > 0.0 {
            self.countdown_left = (self.countdown_left - dt).max(0.0);
            if self.countdown_left <= 0.0 {
                self.events.push(Event {
                    kind: "match_start".to_owned(),
                    id: 0,
                });
            } else {
                return;
            }
        }

        if !self.finished {
            self.time_left -= dt;
            if self.time_left <= 0.0 {
                self.time_left = 0.0;
                self.finished = true;
                self.events.push(Event {
                    kind: "time_up".to_owned(),
                    id: 0,
                });
            }
        }

        {
            let (pellets, tokens, events, inputs, players, time_left) = (
                &mut self.pellets,
                &mut self.tokens,
                &mut self.events,
                &mut self.inputs,
                &mut self.players,
                &mut self.time_left,
            );
            for (session_id, player) in players.iter_mut() {
                if !player.alive {
                    continue;
                }
                let input = inputs.get(session_id).cloned().unwrap_or_default();
                let desired_dir = if input.dir.length_squared() > 0.0001 {
                    input.dir.normalize()
                } else {
                    player.snake.dir()
                };
                let size_t = ((player.snake.radius - BASE_SNAKE_RADIUS)
                    / (MAX_SNAKE_RADIUS - BASE_SNAKE_RADIUS))
                    .clamp(0.0, 1.0);
                let size_speed_mult =
                    SMALL_SNAKE_SPEED_MULT + (1.0 - SMALL_SNAKE_SPEED_MULT) * size_t;
                let token_mult = if player.speedup_left > 0.0 { SPEEDUP_MULT } else { 1.0 };
                let boost_mult = if input.boost && player.boost_energy > 0.01 {
                    BOOST_SPEED_MULT
                } else {
                    1.0
                };
                player.snake.speed = BASE_SPEED * size_speed_mult * token_mult * boost_mult;
                player.snake.update_dir(dt, desired_dir);

                let max_r = (ARENA_RADIUS - player.snake.radius).max(0.0);
                let head = player.snake.head_pos();
                let d = head.length();
                if d > max_r {
                    let clamped = head / d * max_r;
                    let radius = player.snake.radius;
                    let spacing = player.snake.segment_spacing;
                    let target_length = player.snake.target_length;
                    player.snake.reset_at(clamped, desired_dir);
                    player.snake.radius = radius;
                    player.snake.segment_spacing = spacing;
                    player.snake.target_length = target_length;
                }

                if input.boost && player.boost_energy > 0.01 {
                    player.boost_energy =
                        (player.boost_energy - BOOST_ENERGY_DRAIN_PER_SEC * dt).max(0.0);
                } else {
                    player.boost_energy =
                        (player.boost_energy + BOOST_ENERGY_REGEN_PER_SEC * dt).min(BOOST_ENERGY_MAX);
                }

                let size_factor = (BASE_SNAKE_RADIUS / player.snake.radius).clamp(0.25, 1.0);
                let pickup_bonus = if player.magnet_left > 0.0 {
                    MAGNET_PICKUP_BONUS_PX * size_factor
                } else {
                    0.0
                };

                if player.magnet_left > 0.0 {
                    let attract_radius = MAGNET_ATTRACT_RADIUS * (0.55 + 0.45 * size_factor);
                    let attract_speed = MAGNET_ATTRACT_SPEED * (0.75 + 0.25 * size_factor);
                    let attract_max =
                        ((MAGNET_ATTRACT_MAX_PER_FRAME as f32) * (0.35 + 0.65 * size_factor))
                            .round()
                            .clamp(40.0, MAGNET_ATTRACT_MAX_PER_FRAME as f32)
                            as usize;
                    pellets.apply_magnet(
                        player.snake.head_pos(),
                        dt,
                        attract_radius,
                        attract_speed,
                        attract_max,
                    );
                }

                let max_eat = if player.magnet_left > 0.0 {
                    (PELLET_EAT_MAX_PER_FRAME / 2).max(4)
                } else {
                    PELLET_EAT_MAX_PER_FRAME
                };

                let gained = pellets.eat_colliding(
                    player.snake.head_pos(),
                    player.snake.radius,
                    pickup_bonus,
                    max_eat,
                );
                if gained != 0 {
                    player.score += gained;
                }

                let extra = (player.score / SCORE_PER_SEGMENT).max(0) as usize;
                player.snake.target_length =
                    (BASE_SNAKE_LENGTH + extra).clamp(BASE_SNAKE_LENGTH, 900);

                let s = (player.score as f32).max(0.0);
                let t = if s <= 0.0 {
                    0.0
                } else {
                    (s / (s + SNAKE_RADIUS_SCORE_HALF)).clamp(0.0, 1.0)
                };
                let target_radius = (BASE_SNAKE_RADIUS
                    + (MAX_SNAKE_RADIUS - BASE_SNAKE_RADIUS) * t.powf(SNAKE_RADIUS_GROWTH_EXP))
                    .clamp(BASE_SNAKE_RADIUS, MAX_SNAKE_RADIUS);
                let smooth = 1.0 - (-8.0 * dt).exp();
                player.snake.radius =
                    player.snake.radius + (target_radius - player.snake.radius) * smooth;
                let target_spacing = (player.snake.radius * SNAKE_SPACING_MULT)
                    .max((player.snake.radius * 0.78).max(5.2))
                    .min(SNAKE_SPACING_MAX);
                player.snake.segment_spacing = player.snake.segment_spacing
                    + (target_spacing - player.snake.segment_spacing) * smooth;

                let collected = tokens.collect_colliding_filtered(
                    player.snake.head_pos(),
                    player.snake.radius,
                    |_| true,
                );
                for k in collected {
                    match k {
                        TokenKind::Magnet => {
                            player.magnet_left = TOKEN_DURATION_SEC;
                            events.push(Event {
                                kind: "magnet".to_owned(),
                                id: player.id,
                            });
                        }
                        TokenKind::SpeedUp => {
                            player.speedup_left = TOKEN_DURATION_SEC;
                            events.push(Event {
                                kind: "speedup".to_owned(),
                                id: player.id,
                            });
                        }
                        TokenKind::TimeAdd => {
                            *time_left += TOKEN_TIME_ADD_SEC;
                            events.push(Event {
                                kind: "time_add".to_owned(),
                                id: TOKEN_TIME_ADD_SEC as u32,
                            });
                        }
                    }
                }

                player.magnet_left = (player.magnet_left - dt).max(0.0);
                player.speedup_left = (player.speedup_left - dt).max(0.0);
            }
        }

        // Arena bounds (authoritative): mark if outside arena
        let mut dead_ids: HashSet<u32> = HashSet::new();
        for player in self.players.values() {
            if !player.alive {
                continue;
            }
            if player.snake.head_pos().length() > (ARENA_RADIUS - player.snake.radius).max(0.0) {
                dead_ids.insert(player.id);
            }
        }

        // Head-to-head collisions (simple)
        let snapshots: Vec<(u32, Vec2, f32, bool, i32)> = self
            .players
            .values()
            .map(|p| (p.id, p.snake.head_pos(), p.snake.radius, p.alive, p.score))
            .collect();
        let mut to_kill: HashSet<u32> = HashSet::new();
        for i in 0..snapshots.len() {
            for j in (i + 1)..snapshots.len() {
                let (id_a, pos_a, r_a, alive_a, score_a) = snapshots[i];
                let (id_b, pos_b, r_b, alive_b, score_b) = snapshots[j];
                if !alive_a || !alive_b {
                    continue;
                }
                let r = r_a + r_b;
                if pos_a.distance_squared(pos_b) <= r * r {
                    if score_a == score_b {
                        to_kill.insert(id_a);
                        to_kill.insert(id_b);
                    } else if score_a > score_b {
                        to_kill.insert(id_b);
                    } else {
                        to_kill.insert(id_a);
                    }
                }
            }
        }

        // Head-to-body collisions
        let head_snapshots: Vec<(u32, Vec2, f32, bool)> = self
            .players
            .values()
            .map(|p| (p.id, p.snake.head_pos(), p.snake.radius, p.alive))
            .collect();
        let body_snapshots: Vec<(u32, bool, f32, Vec<Vec2>)> = self
            .players
            .values()
            .map(|p| (p.id, p.alive, p.snake.radius, p.snake.segments().to_vec()))
            .collect();

        for (attacker_id, head, head_r, alive) in head_snapshots {
            if !alive {
                continue;
            }
            for (victim_id, victim_alive, victim_r, segments) in &body_snapshots {
                if !victim_alive || *victim_id == attacker_id {
                    continue;
                }
                for (idx, seg) in segments.iter().enumerate() {
                    if idx == 0 {
                        continue;
                    }
                    let r = head_r + *victim_r;
                    if head.distance_squared(*seg) <= r * r {
                        to_kill.insert(attacker_id);
                        break;
                    }
                }
            }
        }

        dead_ids.extend(to_kill.iter().copied());

        if !dead_ids.is_empty() {
            let ids: Vec<u32> = dead_ids.iter().copied().collect();
            let (pellets, events, players) = (&mut self.pellets, &mut self.events, &mut self.players);
            for id in ids {
                if let Some(player) = players.values_mut().find(|p| p.id == id) {
                    kill_player_in_place(pellets, events, player);
                }
            }
        }

        if self.pellets.total() < PELLET_TARGET_COUNT {
            self.pellets.refill_to(PELLET_TARGET_COUNT, PELLET_RADIUS);
        }
        if self.tokens.total() < TOKEN_TARGET_COUNT {
            self.tokens.refill_to_target();
        }
    }

    pub fn player_states(&self) -> Vec<PlayerState> {
        self.players
            .values()
            .map(|id| PlayerState {
                id: id.id,
                alive: id.alive,
                head: Vec2f {
                    x: id.snake.head_pos().x,
                    y: id.snake.head_pos().y,
                },
                dir: Vec2f {
                    x: id.snake.dir().x,
                    y: id.snake.dir().y,
                },
                radius: id.snake.radius,
                score: id.score,
                boost: id.boost_energy,
            })
            .collect()
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        std::mem::swap(&mut out, &mut self.events);
        out
    }

    pub fn time_left(&self) -> f32 {
        self.time_left
    }

    pub fn countdown_left(&self) -> f32 {
        self.countdown_left
    }
}

#[derive(Clone, Copy)]
pub struct InputState {
    pub dir: Vec2,
    pub boost: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            dir: vec2(0.0, 0.0),
            boost: false,
        }
    }
}

pub struct PlayerEntity {
    pub id: u32,
    pub alive: bool,
    pub snake: SnakeSim,
    pub score: i32,
    pub boost_energy: f32,
    pub magnet_left: f32,
    pub speedup_left: f32,
}

fn kill_player_in_place(pellets: &mut Pellets, events: &mut Vec<Event>, player: &mut PlayerEntity) {
    if !player.alive {
        return;
    }
    pellets.spawn_corpse_score(
        player.snake.segments(),
        player.score.max(0),
        CORPSE_DROP_MAX_PELLETS,
        CORPSE_DROP_SPREAD_PX.max(2.0),
    );
    player.alive = false;
    player.magnet_left = 0.0;
    player.speedup_left = 0.0;
    events.push(Event {
        kind: "death".to_owned(),
        id: player.id,
    });
}

fn random_pos_in_disk(radius: f32) -> Vec2 {
    let a = gen_range(0.0f32, std::f32::consts::TAU);
    let r = gen_range(0.0f32, 1.0f32).sqrt() * radius;
    vec2(a.cos() * r, a.sin() * r)
}

fn random_unit_dir() -> Vec2 {
    let a = gen_range(0.0f32, std::f32::consts::TAU);
    vec2(a.cos(), a.sin())
}
