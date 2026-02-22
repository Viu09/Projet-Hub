use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::config::{ARENA_RADIUS, BOOST_ENERGY_MAX, DEMO_BOT_COUNT};
use crate::game::snake_sim::SnakeSim;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Running,
    Spectating,
    Finished,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    TimeUp,
    AllEliminated,
    LastAlive,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Player,
    Bot,
}

pub struct Agent {
    pub kind: AgentKind,
    pub name: String,
    pub color_head: Color,
    pub color_body: Color,

    pub snake: SnakeSim,
    pub alive: bool,
    #[allow(dead_code)]
    pub respawn_left: f32,

    pub score: f32,
    pub boost_energy: f32,

    pub magnet_left: f32,
    pub speedup_left: f32,

    // Bot state
    pub bot_dir: Vec2,
    pub bot_boost_intent: f32,
    pub bot_hunt_target: Option<usize>,
    pub bot_hunt_left: f32,
}

#[derive(Clone, Copy)]
pub struct AgentSnapshot {
    pub alive: bool,
    pub head: Vec2,
    pub radius: f32,
}

pub struct FrameScratch {
    pub agents_snapshot: Vec<AgentSnapshot>,
    pub to_die: Vec<bool>,
    pub leaderboard_order: Vec<usize>,
}

impl FrameScratch {
    pub fn new() -> Self {
        Self {
            agents_snapshot: Vec::new(),
            to_die: Vec::new(),
            leaderboard_order: Vec::new(),
        }
    }

    pub fn resize_for_agents(&mut self, agents_len: usize) {
        self.agents_snapshot.clear();
        self.agents_snapshot.reserve(agents_len);

        if self.to_die.len() != agents_len {
            self.to_die.resize(agents_len, false);
        }
        self.to_die.fill(false);

        self.leaderboard_order.clear();
        self.leaderboard_order.reserve(agents_len);
    }
}

fn random_pos_in_disk(radius: f32) -> Vec2 {
    let a = gen_range(0.0f32, std::f32::consts::TAU);
    let r = gen_range(0.0f32, 1.0f32).sqrt() * radius;
    vec2(a.cos() * r, a.sin() * r)
}

pub fn random_unit_dir() -> Vec2 {
    let a = gen_range(0.0f32, std::f32::consts::TAU);
    vec2(a.cos(), a.sin())
}

fn pick_spawn_pos(agents: &[Agent], arena_radius: f32) -> Vec2 {
    for _ in 0..30 {
        let p = random_pos_in_disk(arena_radius * 0.70);
        let mut ok = true;
        for a in agents {
            if a.alive && a.snake.head_pos().distance_squared(p) < 260.0 * 260.0 {
                ok = false;
                break;
            }
        }
        if ok {
            return p;
        }
    }
    random_pos_in_disk(arena_radius * 0.60)
}

pub fn make_initial_agents() -> Vec<Agent> {
    let mut agents: Vec<Agent> = Vec::new();

    let spectator_demo = cfg!(feature = "demo100");
    let heavy_mode = cfg!(feature = "demo100") || cfg!(feature = "demo_play100");
    let bot_count: usize = if heavy_mode { DEMO_BOT_COUNT } else { 5 };

    if !spectator_demo {
        let player_spawn = vec2(0.0, 0.0);
        agents.push(Agent {
            kind: AgentKind::Player,
            name: "YOU".to_owned(),
            color_head: YELLOW,
            color_body: ORANGE,
            snake: SnakeSim::new_at(player_spawn, vec2(1.0, 0.0)),
            alive: true,
            respawn_left: 0.0,
            score: 0.0,
            boost_energy: BOOST_ENERGY_MAX,
            magnet_left: 0.0,
            speedup_left: 0.0,
            bot_dir: vec2(1.0, 0.0),
            bot_boost_intent: 0.0,
            bot_hunt_target: None,
            bot_hunt_left: 0.0,
        });
    }

    let palette = [
        (Color::from_rgba(255, 140, 90, 255), Color::from_rgba(255, 120, 60, 255)),
        (Color::from_rgba(110, 220, 255, 255), Color::from_rgba(80, 180, 240, 255)),
        (Color::from_rgba(170, 255, 130, 255), Color::from_rgba(120, 220, 90, 255)),
        (Color::from_rgba(255, 120, 200, 255), Color::from_rgba(220, 90, 180, 255)),
        (Color::from_rgba(220, 220, 255, 255), Color::from_rgba(180, 180, 240, 255)),
        (Color::from_rgba(255, 210, 120, 255), Color::from_rgba(255, 190, 80, 255)),
    ];

    for i in 0..bot_count {
        let spawn = pick_spawn_pos(&agents, ARENA_RADIUS);
        let dir = random_unit_dir();
        let (head, body) = palette[i % palette.len()];
        agents.push(Agent {
            kind: AgentKind::Bot,
            name: format!("BOT{}", i + 1),
            color_head: head,
            color_body: body,
            snake: SnakeSim::new_at(spawn, dir),
            alive: true,
            respawn_left: 0.0,
            score: 0.0,
            boost_energy: BOOST_ENERGY_MAX,
            magnet_left: 0.0,
            speedup_left: 0.0,
            bot_dir: dir,
            bot_boost_intent: 0.0,
            bot_hunt_target: None,
            bot_hunt_left: 0.0,
        });
    }

    agents
}
