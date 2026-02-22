use macroquad::prelude::*;

use crate::config::{ARENA_RADIUS, CORPSE_DROP_MAX_PELLETS, CORPSE_DROP_SPREAD_PX};
use crate::game::food::Pellets;
use crate::game::world::{Agent, AgentKind, FrameScratch};

pub fn check_arena_bounds(agents: &[Agent], scratch: &mut FrameScratch) {
    for i in 0..agents.len() {
        if !agents[i].alive {
            continue;
        }
        if agents[i].snake.head_pos().distance_squared(vec2(0.0, 0.0)) > ARENA_RADIUS * ARENA_RADIUS {
            scratch.to_die[i] = true;
        }
    }
}

pub fn check_head_to_head(agents: &[Agent], scratch: &mut FrameScratch) {
    for i in 0..agents.len() {
        if !agents[i].alive || scratch.to_die[i] {
            continue;
        }
        for j in (i + 1)..agents.len() {
            if !agents[j].alive || scratch.to_die[j] {
                continue;
            }
            let pi = agents[i].snake.head_pos();
            let pj = agents[j].snake.head_pos();
            let r = agents[i].snake.radius + agents[j].snake.radius;
            if pi.distance_squared(pj) <= (r * 0.95) * (r * 0.95) {
                let ai = agents[i].snake.radius.max(0.01);
                let aj = agents[j].snake.radius.max(0.01);
                let ratio = if ai > aj { ai / aj } else { aj / ai };
                if ratio < 1.10 {
                    scratch.to_die[i] = true;
                    scratch.to_die[j] = true;
                } else if ai > aj {
                    scratch.to_die[j] = true;
                } else {
                    scratch.to_die[i] = true;
                }
            }
        }
    }
}

pub fn check_head_to_body(agents: &[Agent], scratch: &mut FrameScratch, heavy_mode: bool) {
    for i in 0..agents.len() {
        if !agents[i].alive || scratch.to_die[i] {
            continue;
        }
        let head = agents[i].snake.head_pos();
        let hr = agents[i].snake.radius;
        'outer: for j in 0..agents.len() {
            if i == j || !agents[j].alive {
                continue;
            }
            let br = agents[j].snake.radius;
            let segs = agents[j].snake.segments();
            let step = if heavy_mode {
                (segs.len() / 110).max(1).min(6)
            } else {
                1
            };

            let sample_padding = if heavy_mode {
                ((step as f32) - 1.0).max(0.0) * agents[j].snake.segment_spacing * 0.55
            } else {
                0.0
            };

            let r = (hr + br * 0.92 + sample_padding).max(0.0);
            let r2 = r * r;
            let mut k: usize = 3;
            while k < segs.len() {
                let p = segs[k];
                if head.distance_squared(p) <= r2 {
                    scratch.to_die[i] = true;
                    break 'outer;
                }
                k += step;
            }
        }
    }
}

pub fn apply_deaths(agents: &mut [Agent], scratch: &mut FrameScratch, pellets: &mut Pellets) -> bool {
    let mut player_died = false;

    for i in 0..agents.len() {
        if !agents[i].alive || !scratch.to_die[i] {
            continue;
        }

        pellets.spawn_corpse_score(
            agents[i].snake.segments(),
            agents[i].score as i32,
            CORPSE_DROP_MAX_PELLETS,
            CORPSE_DROP_SPREAD_PX.max(2.0),
        );

        agents[i].alive = false;
        agents[i].magnet_left = 0.0;
        agents[i].speedup_left = 0.0;

        if agents[i].kind == AgentKind::Player {
            player_died = true;
        }
    }

    player_died
}
