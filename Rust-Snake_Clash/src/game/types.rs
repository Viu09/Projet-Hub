use macroquad::prelude::*;

use crate::entities::Snake;

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

    pub snake: Snake,
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
