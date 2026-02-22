use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::config::{
    ARENA_RADIUS, BASE_SNAKE_LENGTH, BASE_SPEED, BOOST_ENERGY_DRAIN_PER_SEC, BOOST_ENERGY_MAX,
    BOOST_ENERGY_REGEN_PER_SEC, BOOST_SPEED_MULT, ENERGY_BAR_MAX,
    ISO_SCALE, MATCH_DURATION_SEC, PELLET_BUCKET_SIZE, PELLET_RADIUS, PELLET_TARGET_COUNT,
    SCORE_PER_SEGMENT, UI_BOOST_BUTTON_CENTER, UI_BOOST_BUTTON_RADIUS, UI_JOYSTICK_CENTER,
    MAGNET_ATTRACT_MAX_PER_FRAME, MAGNET_ATTRACT_RADIUS, MAGNET_ATTRACT_SPEED, MAGNET_PICKUP_BONUS_PX,
    SPEEDUP_MULT, TOKEN_DURATION_SEC, TOKEN_TARGET_COUNT, TOKEN_TIME_ADD_SEC,
    UI_JOYSTICK_DEADZONE, UI_JOYSTICK_RADIUS,
    BASE_SNAKE_RADIUS, MAX_SNAKE_RADIUS, SNAKE_RADIUS_GROWTH_EXP, SNAKE_RADIUS_SCORE_HALF,
    SNAKE_SPACING_MAX, SNAKE_SPACING_MULT,
    CAMERA_CENTER_BODY_BLEND, CAMERA_FIT_SCREEN_FRACTION, CAMERA_SCALE_BASE, CAMERA_SCALE_MAX, CAMERA_SCALE_MIN,
    PELLET_EAT_MAX_PER_FRAME,
    SMALL_SNAKE_SPEED_MULT,
    CAMERA_SNAKE_EXTENT_WORLD_MIN, CAMERA_SNAKE_EXTENT_WORLD_MULT,
    SPECTATE_CAMERA_CLAMP_MULT, SPECTATE_PAN_BOOST_MULT, SPECTATE_PAN_SPEED,
    SPECTATE_ZOOM_MAX, SPECTATE_ZOOM_MIN, SPECTATE_ZOOM_WHEEL_SENS,
    UI_ZOOM_BUTTON_RADIUS,
    UI_SCALE,
};
use crate::game::collision::{
    apply_deaths, check_arena_bounds, check_head_to_body, check_head_to_head,
};
use crate::game::food::{draw_token_screen, Pellets, TokenKind, Tokens};
use crate::game::math::{
    input_pos_scale, point_in_circle, screen_to_world, screen_vec_to_world_dir, ui_anchor_portrait, world_to_screen,
};
use crate::game::world::{
    make_initial_agents, random_unit_dir, AgentKind, FinishReason, FrameScratch, RunState,
};
use crate::game::snake_sim::SnakeSim;
use crate::client::runtime;

fn draw_token_badge(x: f32, y: f32, kind: TokenKind, seconds_left: f32) {
    let s = UI_SCALE;
    let w = 132.0 * s;
    let h = 34.0 * s;
    draw_rectangle(x, y, w, h, Color::from_rgba(0, 0, 0, 70));
    draw_rectangle_lines(x, y, w, h, 2.0 * s, Color::from_rgba(255, 255, 255, 35));

    let icon_center = vec2(x + 18.0 * s, y + h * 0.5);
    draw_token_screen(icon_center, 10.0 * s, kind);

    let label = match kind {
        TokenKind::Magnet => "MAGNET",
        TokenKind::SpeedUp => "SPEED",
        TokenKind::TimeAdd => "TIME",
    };
    draw_text(label, x + 36.0 * s, y + 22.0 * s, 18.0 * s, Color::from_rgba(255, 255, 255, 220));
    draw_text(
        &format!("{:04.1}", seconds_left.max(0.0)),
        x + w - 44.0 * s,
        y + 22.0 * s,
        18.0 * s,
        Color::from_rgba(255, 255, 255, 190),
    );
}

fn agents_from_players(players: &[crate::net::messages::PlayerState]) -> Vec<crate::game::world::Agent> {
    let palette = [
        (Color::from_rgba(255, 140, 90, 255), Color::from_rgba(255, 120, 60, 255)),
        (Color::from_rgba(110, 220, 255, 255), Color::from_rgba(80, 180, 240, 255)),
        (Color::from_rgba(170, 255, 130, 255), Color::from_rgba(120, 220, 90, 255)),
        (Color::from_rgba(255, 120, 200, 255), Color::from_rgba(220, 90, 180, 255)),
        (Color::from_rgba(220, 220, 255, 255), Color::from_rgba(180, 180, 240, 255)),
        (Color::from_rgba(255, 210, 120, 255), Color::from_rgba(255, 190, 80, 255)),
    ];

    players
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let (head, body) = palette[idx % palette.len()];
            let head_pos = vec2(p.head.x, p.head.y);
            let dir = vec2(p.dir.x, p.dir.y);
            let mut snake = SnakeSim::new_at(head_pos, dir);
            snake.radius = p.radius;
            crate::game::world::Agent {
                kind: AgentKind::Player,
                name: format!("P{}", p.id),
                color_head: head,
                color_body: body,
                snake,
                alive: p.alive,
                respawn_left: 0.0,
                score: p.score as f32,
                boost_energy: p.boost,
                magnet_left: 0.0,
                speedup_left: 0.0,
                bot_dir: dir,
                bot_boost_intent: 0.0,
                bot_hunt_target: None,
                bot_hunt_left: 0.0,
            }
        })
        .collect()
}

fn token_kind_from_str(value: &str) -> Option<TokenKind> {
    match value {
        "magnet" => Some(TokenKind::Magnet),
        "speed" => Some(TokenKind::SpeedUp),
        "time" => Some(TokenKind::TimeAdd),
        _ => None,
    }
}

fn agent_id_from_name(name: &str) -> u32 {
    name.strip_prefix('P')
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}

pub async fn run() {
    let mut agents = make_initial_agents();
    let mut scratch = FrameScratch::new();

    let mut pellets = Pellets::new(PELLET_BUCKET_SIZE, ARENA_RADIUS);
    pellets.populate_random(PELLET_TARGET_COUNT, PELLET_RADIUS);

    let mut tokens = Tokens::new(ARENA_RADIUS, TOKEN_TARGET_COUNT);
    tokens.populate_random();

    let arena_center = vec2(0.0, 0.0);
    let mut time_left: f32 = MATCH_DURATION_SEC;
    let mut time_added_total: f32 = 0.0;
    let mut time_add_flash: f32 = 0.0;
    let mut countdown_left: f32 = 0.0;
    let spectator_demo = cfg!(feature = "demo100");
    let mut state = if spectator_demo {
        RunState::Spectating
    } else {
        RunState::Running
    };
    let mut finish_reason: Option<FinishReason> = None;

    let mut best_score: i32 = 0;

    let mut finished_winner_idx: Option<usize> = None;

    let mut demo_restart_left: f32 = 0.0;

    let mut timeadd_badge_left: f32 = 0.0;
    let mut toast_left: f32 = 0.0;
    let mut toast_text: String = String::new();
    let mut net_magnet_left: f32 = 0.0;
    let mut net_speedup_left: f32 = 0.0;

    let mut joystick_active = false;
    let mut joystick_origin: Vec2;

    let mut camera_center = agents[0].snake.head_pos();
    let mut camera_scale = CAMERA_SCALE_BASE;
    let mut camera_extent_smoothed: f32 = 1.0;

    let mut last_player_pos = agents[0].snake.head_pos();

    let mut spectate_zoom: f32 = 1.0;
    let mut prev_pinch_dist: Option<f32> = None;

    loop {
        if crate::client::lobby_ui::update() {
            next_frame().await;
            continue;
        }
        runtime::poll();
        let dt = get_frame_time();
        clear_background(Color::from_rgba(12, 14, 20, 255));

        let spectator_demo = cfg!(feature = "demo100");
        let heavy_mode = cfg!(feature = "demo100") || cfg!(feature = "demo_play100");

        let input_scale = input_pos_scale();

        let ui_s = UI_SCALE;
        let joystick_radius = UI_JOYSTICK_RADIUS * ui_s;
        let boost_radius = UI_BOOST_BUTTON_RADIUS * ui_s;
        let zoom_btn_radius = UI_ZOOM_BUTTON_RADIUS * ui_s;

        let mm_size = 170.0 * ui_s;
        let mm_x = screen_width() - mm_size - 16.0 * ui_s;
        let mm_y = 16.0 * ui_s + 212.0 * ui_s;

        joystick_origin = ui_anchor_portrait(UI_JOYSTICK_CENTER);
        let boost_btn_center = ui_anchor_portrait(UI_BOOST_BUTTON_CENTER);

        let zoom_x = mm_x + mm_size - zoom_btn_radius - 6.0 * ui_s;
        let zoom_plus_center = vec2(zoom_x, mm_y + mm_size + 14.0 * ui_s + zoom_btn_radius);
        let zoom_minus_center = vec2(zoom_x, zoom_plus_center.y + zoom_btn_radius * 2.0 + 12.0 * ui_s);

        let mut zoom_plus_held = false;
        let mut zoom_minus_held = false;

        let screen_center = vec2(screen_width() * 0.5, screen_height() * 0.56);

        let mut stick_delta = vec2(0.0, 0.0);
        let mut stick_has_input = false;

        if let Some(t) = touches().first() {
            let p = t.position * input_scale;
            if !joystick_active {
                if p.x > screen_width() * 0.45 {
                    joystick_active = true;
                }
            }
            if joystick_active {
                stick_delta = p - joystick_origin;
                stick_has_input = true;
            }
        } else {
            joystick_active = false;
        }

        if !stick_has_input && is_mouse_button_down(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let p = vec2(mx, my) * input_scale;
            if p.x > screen_width() * 0.45 {
                stick_delta = p - joystick_origin;
                stick_has_input = true;
            }
        }

        let stick_len = stick_delta.length();
        let stick_dir_screen = if stick_len > 0.001 {
            let clamped = stick_delta / stick_len * stick_len.min(joystick_radius);
            clamped
        } else {
            vec2(0.0, 0.0)
        };

        let mut desired_dir_world = vec2(0.0, 0.0);
        let deadzone_px = joystick_radius * UI_JOYSTICK_DEADZONE;
        if stick_has_input && stick_dir_screen.length() > deadzone_px {
            desired_dir_world = screen_vec_to_world_dir(stick_dir_screen, camera_scale);
        }

        let wants_boost_keyboard = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let wants_boost_mouse = is_mouse_button_down(MouseButton::Right);
        let mut wants_boost_touch = false;
        for t in touches() {
            if point_in_circle(t.position * input_scale, boost_btn_center, boost_radius) {
                wants_boost_touch = true;
                break;
            }
        }
        let wants_boost = wants_boost_keyboard || wants_boost_mouse || wants_boost_touch;
        runtime::send_input(
            crate::net::messages::Vec2f {
                x: desired_dir_world.x,
                y: desired_dir_world.y,
            },
            wants_boost,
        );

        let mut net_mode = false;
        let mut net_agents: Option<Vec<crate::game::world::Agent>> = None;
        {
            let mut players = runtime::latest_players();
            if !players.is_empty() {
                if let Some(local_id) = runtime::local_player_id() {
                    let mut reordered = Vec::with_capacity(players.len());
                    if let Some(pos) = players.iter().position(|p| p.id == local_id) {
                        reordered.push(players[pos].clone());
                        for (idx, p) in players.iter().enumerate() {
                            if idx != pos {
                                reordered.push(p.clone());
                            }
                        }
                        players = reordered;
                    }
                }
                net_agents = Some(agents_from_players(&players));
                net_mode = true;
            }
        }

        if net_mode {
            time_left = runtime::latest_time_left();
            countdown_left = runtime::latest_countdown_left();
        }

        if state == RunState::Spectating {
            let (_wx, wy) = mouse_wheel();
            if wy.abs() > 0.001 {
                spectate_zoom *= (1.0 + wy * SPECTATE_ZOOM_WHEEL_SENS).clamp(0.85, 1.25);
            }

            for t in touches() {
                let p = t.position * input_scale;
                if point_in_circle(p, zoom_plus_center, zoom_btn_radius) {
                    zoom_plus_held = true;
                }
                if point_in_circle(p, zoom_minus_center, zoom_btn_radius) {
                    zoom_minus_held = true;
                }
            }
            if !zoom_plus_held && !zoom_minus_held && is_mouse_button_down(MouseButton::Left) {
                let (mx, my) = mouse_position();
                let p = vec2(mx, my) * input_scale;
                zoom_plus_held = point_in_circle(p, zoom_plus_center, zoom_btn_radius);
                zoom_minus_held = point_in_circle(p, zoom_minus_center, zoom_btn_radius);
            }
            if zoom_plus_held {
                spectate_zoom *= 1.0 + 1.6 * dt;
            }
            if zoom_minus_held {
                spectate_zoom *= 1.0 - 1.6 * dt;
            }

            if is_key_down(KeyCode::Equal) || is_key_down(KeyCode::KpAdd) {
                spectate_zoom *= 1.0 + 1.4 * dt;
            }
            if is_key_down(KeyCode::Minus) || is_key_down(KeyCode::KpSubtract) {
                spectate_zoom *= 1.0 - 1.4 * dt;
            }

            let ts = touches();
            if ts.len() >= 2 {
                let p0 = ts[0].position * input_scale;
                let p1 = ts[1].position * input_scale;

                let joystick_r = joystick_radius * 1.25;
                let in_joystick = point_in_circle(p0, joystick_origin, joystick_r)
                    || point_in_circle(p1, joystick_origin, joystick_r);
                let in_boost = point_in_circle(p0, boost_btn_center, boost_radius * 1.10)
                    || point_in_circle(p1, boost_btn_center, boost_radius * 1.10);
                let in_zoom_btn = point_in_circle(p0, zoom_plus_center, zoom_btn_radius * 1.10)
                    || point_in_circle(p1, zoom_plus_center, zoom_btn_radius * 1.10)
                    || point_in_circle(p0, zoom_minus_center, zoom_btn_radius * 1.10)
                    || point_in_circle(p1, zoom_minus_center, zoom_btn_radius * 1.10);

                let allow_pinch = !(in_joystick || in_boost || in_zoom_btn);

                if !allow_pinch {
                    prev_pinch_dist = None;
                } else {
                    let d = p0.distance(p1);
                    if let Some(prev) = prev_pinch_dist {
                        if prev > 1.0 {
                            let ratio = (d / prev).clamp(0.85, 1.18);
                            spectate_zoom *= ratio;
                        }
                    }
                    prev_pinch_dist = Some(d);
                }
            } else {
                prev_pinch_dist = None;
            }

            spectate_zoom = spectate_zoom.clamp(SPECTATE_ZOOM_MIN, SPECTATE_ZOOM_MAX);
        } else {
            spectate_zoom = 1.0;
            prev_pinch_dist = None;
        }

        if net_mode {
            if time_left <= 0.0 {
                state = RunState::Finished;
                finish_reason = Some(FinishReason::TimeUp);
            }
            timeadd_badge_left = (timeadd_badge_left - dt).max(0.0);
            toast_left = (toast_left - dt).max(0.0);
            time_add_flash = (time_add_flash - dt).max(0.0);
        } else {
            if state != RunState::Finished {
                time_left -= dt;
                if time_left <= 0.0 {
                    time_left = 0.0;
                    state = RunState::Finished;
                    finish_reason = Some(FinishReason::TimeUp);
                    finished_winner_idx = None;
                    if spectator_demo {
                        demo_restart_left = 2.0;
                    }
                }
            }

            if state != RunState::Finished {
                timeadd_badge_left = (timeadd_badge_left - dt).max(0.0);
                toast_left = (toast_left - dt).max(0.0);
                time_add_flash = (time_add_flash - dt).max(0.0);

                for a in &mut agents {
                    a.magnet_left = (a.magnet_left - dt).max(0.0);
                    a.speedup_left = (a.speedup_left - dt).max(0.0);
                }

                scratch.resize_for_agents(agents.len());
                for a in &agents {
                    scratch.agents_snapshot.push(crate::game::world::AgentSnapshot {
                        alive: a.alive,
                        head: a.snake.head_pos(),
                        radius: a.snake.radius,
                    });
                }
                let alive_count_snapshot = scratch.agents_snapshot.iter().filter(|s| s.alive).count();

                for idx in 0..agents.len() {
                    if !agents[idx].alive {
                        continue;
                    }

                    let is_player = agents[idx].kind == AgentKind::Player;
                    if is_player && state != RunState::Running {
                        continue;
                    }

                    let (desired, wants_boost_agent) = if is_player {
                        (desired_dir_world, wants_boost)
                    } else {
                        let norm = |v: Vec2| {
                            if v.length_squared() > 0.0001 {
                                v.normalize()
                            } else {
                                vec2(0.0, 0.0)
                            }
                        };

                    let head = scratch.agents_snapshot[idx].head;
                    let my_r = scratch.agents_snapshot[idx].radius;

                    let late_game = if heavy_mode {
                        alive_count_snapshot <= 18
                    } else {
                        alive_count_snapshot <= 7
                    };
                    let big_enough = if heavy_mode { my_r >= 18.0 } else { my_r >= 22.0 };

                    agents[idx].bot_hunt_left = (agents[idx].bot_hunt_left - dt).max(0.0);
                    let mut hunt: Option<(usize, Vec2, f32)> = None;

                    if agents[idx].bot_hunt_left > 0.0 {
                        if let Some(ti) = agents[idx].bot_hunt_target {
                            if ti != idx {
                                let s = scratch.agents_snapshot[ti];
                                if s.alive && my_r > s.radius * 1.07 {
                                    let dist = head.distance(s.head);
                                    hunt = Some((ti, s.head, dist));
                                }
                            }
                        }
                        if hunt.is_none() {
                            agents[idx].bot_hunt_target = None;
                            agents[idx].bot_hunt_left = 0.0;
                        }
                    }

                    if hunt.is_none() && (late_game || big_enough) {
                        let chase_r = if heavy_mode { 1800.0 } else { 1200.0 };
                        let mut best: Option<(usize, Vec2, f32, f32)> = None;
                        for (j, s) in scratch.agents_snapshot.iter().enumerate() {
                            if !s.alive || j == idx {
                                continue;
                            }
                            if my_r <= s.radius * 1.07 {
                                continue;
                            }
                            let dist = head.distance(s.head);
                            if dist > chase_r {
                                continue;
                            }
                            let ratio = (my_r / (s.radius).max(0.01)).clamp(1.0, 3.0);
                            let score = ratio * 900.0 - dist;
                            if best.as_ref().map(|b| score > b.3).unwrap_or(true) {
                                best = Some((j, s.head, dist, score));
                            }
                        }
                        if let Some((ti, pos, dist, _score)) = best {
                            agents[idx].bot_hunt_target = Some(ti);
                            agents[idx].bot_hunt_left = gen_range(1.2f32, if heavy_mode { 2.8f32 } else { 2.2f32 });
                            hunt = Some((ti, pos, dist));
                        }
                    }

                    let d = head.length();
                    let inward = if d > ARENA_RADIUS * 0.80 {
                        norm(-head)
                    } else {
                        vec2(0.0, 0.0)
                    };

                    let mut flee = vec2(0.0, 0.0);
                    let mut flee_w = 0.0;
                    let danger_r = 620.0;
                    for (j, s) in scratch.agents_snapshot.iter().enumerate() {
                        if !s.alive || j == idx {
                            continue;
                        }
                        if s.radius <= my_r * 1.08 {
                            continue;
                        }
                        let dist = head.distance(s.head);
                        if dist < danger_r {
                            let dir = (head - s.head) / dist.max(0.001);
                            let w = (1.0 - dist / danger_r).clamp(0.0, 1.0);
                            flee += dir * w;
                            flee_w += w;
                        }
                    }

                    let mut chase: Option<(Vec2, f32)> = None;
                    let chase_r = if heavy_mode { 720.0 } else { 560.0 };
                    if flee_w <= 0.01 && hunt.is_none() {
                        let mut best_d = f32::INFINITY;
                        for (j, s) in scratch.agents_snapshot.iter().enumerate() {
                            if !s.alive || j == idx {
                                continue;
                            }
                            if my_r <= s.radius * 1.12 {
                                continue;
                            }
                            let dist = head.distance(s.head);
                            if dist < chase_r && dist < best_d {
                                best_d = dist;
                                chase = Some((s.head, dist));
                            }
                        }
                    }

                    let token_target = tokens.best_target(head, |k| match k {
                        TokenKind::TimeAdd => Some(1.7),
                        TokenKind::Magnet => if agents[idx].magnet_left > 0.0 { None } else { Some(2.2) },
                        TokenKind::SpeedUp => if agents[idx].speedup_left > 0.0 { None } else { Some(2.0) },
                    });

                    let pellet_target = pellets.best_pellet_target(head, 820.0);

                    let (mut desired, wants_boost_agent) = if flee_w > 0.01 {
                        (norm(flee), agents[idx].boost_energy > 30.0)
                    } else if let Some((_ti, tpos, dist)) = hunt {
                        let wants = agents[idx].boost_energy > 35.0 && dist < (if heavy_mode { 980.0 } else { 760.0 });
                        (norm(tpos - head), wants)
                    } else if let Some((tpos, dist)) = chase {
                        (norm(tpos - head), agents[idx].boost_energy > 45.0 && dist < 360.0)
                    } else if let Some((tpos, tkind)) = token_target {
                        let wants = match tkind {
                            TokenKind::TimeAdd => agents[idx].boost_energy > 65.0 && head.distance(tpos) < 520.0,
                            _ => agents[idx].boost_energy > 55.0 && head.distance(tpos) < 520.0,
                        };
                        (norm(tpos - head), wants)
                    } else if let Some(ppos) = pellet_target {
                        (norm(ppos - head), false)
                    } else {
                        let jitter = random_unit_dir();
                        (norm(agents[idx].bot_dir.lerp(jitter, 0.08)), false)
                    };

                    if desired.length_squared() <= 0.0001 {
                        desired = agents[idx].bot_dir;
                    }

                    if inward.length_squared() > 0.0001 {
                        desired = norm(desired * 0.62 + inward * 1.10);
                    }

                    let t = (3.4 * dt).clamp(0.0, 1.0);
                    agents[idx].bot_dir = norm(agents[idx].bot_dir.lerp(desired, 0.55 * t));

                    agents[idx].bot_boost_intent = (agents[idx].bot_boost_intent - dt).max(0.0);
                    if wants_boost_agent {
                        if agents[idx].bot_boost_intent <= 0.0 {
                            agents[idx].bot_boost_intent = gen_range(0.25f32, 0.55f32);
                        }
                    }
                    let wants_boost_agent = agents[idx].bot_boost_intent > 0.0;

                    (agents[idx].bot_dir, wants_boost_agent)
                };

                let boosting = wants_boost_agent && agents[idx].boost_energy > 0.01;

                let size_t = ((agents[idx].snake.radius - BASE_SNAKE_RADIUS)
                    / (MAX_SNAKE_RADIUS - BASE_SNAKE_RADIUS))
                    .clamp(0.0, 1.0);
                let size_speed_mult = SMALL_SNAKE_SPEED_MULT + (1.0 - SMALL_SNAKE_SPEED_MULT) * size_t;
                let token_mult = if agents[idx].speedup_left > 0.0 { SPEEDUP_MULT } else { 1.0 };
                let boost_mult = if boosting { BOOST_SPEED_MULT } else { 1.0 };
                agents[idx].snake.speed = BASE_SPEED * size_speed_mult * token_mult * boost_mult;

                if boosting {
                    agents[idx].boost_energy =
                        (agents[idx].boost_energy - BOOST_ENERGY_DRAIN_PER_SEC * dt).max(0.0);
                } else {
                    agents[idx].boost_energy =
                        (agents[idx].boost_energy + BOOST_ENERGY_REGEN_PER_SEC * dt).min(BOOST_ENERGY_MAX);
                }

                agents[idx].snake.update_dir(dt, desired);
            }

            if agents[0].alive {
                last_player_pos = agents[0].snake.head_pos();
            }

            check_arena_bounds(&agents, &mut scratch);
            check_head_to_head(&agents, &mut scratch);
            check_head_to_body(&agents, &mut scratch, heavy_mode);

            if apply_deaths(&mut agents, &mut scratch, &mut pellets) {
                state = RunState::Spectating;
                toast_left = 0.0;
                timeadd_badge_left = 0.0;
            }

            if agents.iter().all(|a| !a.alive) {
                state = RunState::Finished;
                time_left = 0.0;
                finish_reason = Some(FinishReason::AllEliminated);
                finished_winner_idx = None;
                if spectator_demo {
                    demo_restart_left = 2.0;
                }
            }

            if state != RunState::Finished {
                let alive_count = agents.iter().filter(|a| a.alive).count();
                if alive_count == 1 {
                    state = RunState::Finished;
                    finish_reason = Some(FinishReason::LastAlive);
                    finished_winner_idx = agents.iter().position(|a| a.alive);
                    if spectator_demo {
                        demo_restart_left = 2.0;
                    }
                }
            }

            if state != RunState::Finished {
                for i in 0..agents.len() {
                    if !agents[i].alive {
                        continue;
                    }

                    let collected = tokens.collect_colliding_filtered(
                        agents[i].snake.head_pos(),
                        agents[i].snake.radius,
                        |k| match k {
                            TokenKind::Magnet => agents[i].magnet_left <= 0.0,
                            TokenKind::SpeedUp => agents[i].speedup_left <= 0.0,
                            TokenKind::TimeAdd => true,
                        },
                    );
                    for k in collected {
                        match k {
                            TokenKind::Magnet => {
                                agents[i].magnet_left = TOKEN_DURATION_SEC;
                                if agents[i].kind == AgentKind::Player {
                                    toast_text = "MAGNET (10s)".to_owned();
                                    toast_left = 1.2;
                                }
                            }
                            TokenKind::SpeedUp => {
                                agents[i].speedup_left = TOKEN_DURATION_SEC;
                                if agents[i].kind == AgentKind::Player {
                                    toast_text = "SPEED (10s)".to_owned();
                                    toast_left = 1.2;
                                }
                            }
                            TokenKind::TimeAdd => {
                                time_left += TOKEN_TIME_ADD_SEC;
                                time_added_total += TOKEN_TIME_ADD_SEC;
                                if agents[i].kind == AgentKind::Player {
                                    time_add_flash = 0.9;
                                    timeadd_badge_left = 1.4;
                                    toast_text = "TIME +10s".to_owned();
                                    toast_left = 1.2;
                                }
                            }
                        }
                    }

                    let size_factor = (BASE_SNAKE_RADIUS / agents[i].snake.radius).clamp(0.25, 1.0);
                    let pickup_bonus = if agents[i].magnet_left > 0.0 {
                        MAGNET_PICKUP_BONUS_PX * size_factor
                    } else {
                        0.0
                    };

                    if agents[i].magnet_left > 0.0 {
                        let attract_radius = MAGNET_ATTRACT_RADIUS * (0.55 + 0.45 * size_factor);
                        let attract_speed = MAGNET_ATTRACT_SPEED * (0.75 + 0.25 * size_factor);
                        let attract_max = ((MAGNET_ATTRACT_MAX_PER_FRAME as f32) * (0.35 + 0.65 * size_factor))
                            .round()
                            .clamp(40.0, MAGNET_ATTRACT_MAX_PER_FRAME as f32) as usize;
                        pellets.apply_magnet(agents[i].snake.head_pos(), dt, attract_radius, attract_speed, attract_max);
                    }

                    let max_eat = if agents[i].magnet_left > 0.0 {
                        (PELLET_EAT_MAX_PER_FRAME / 2).max(4)
                    } else {
                        PELLET_EAT_MAX_PER_FRAME
                    };
                    let gained = pellets.eat_colliding(
                        agents[i].snake.head_pos(),
                        agents[i].snake.radius,
                        pickup_bonus,
                        max_eat,
                    );
                    if gained != 0 {
                        agents[i].score += gained as f32;
                    }

                    best_score = best_score.max(agents[i].score as i32);

                    let extra = ((agents[i].score as i32) / SCORE_PER_SEGMENT).max(0) as usize;
                    agents[i].snake.target_length = (BASE_SNAKE_LENGTH + extra).clamp(BASE_SNAKE_LENGTH, 900);

                    let s = agents[i].score.max(0.0);
                    let t = if s <= 0.0 { 0.0 } else { (s / (s + SNAKE_RADIUS_SCORE_HALF)).clamp(0.0, 1.0) };
                    let target_radius = (BASE_SNAKE_RADIUS
                        + (MAX_SNAKE_RADIUS - BASE_SNAKE_RADIUS) * t.powf(SNAKE_RADIUS_GROWTH_EXP))
                        .clamp(BASE_SNAKE_RADIUS, MAX_SNAKE_RADIUS);
                    let smooth = 1.0 - (-8.0 * dt).exp();
                    agents[i].snake.radius = agents[i].snake.radius + (target_radius - agents[i].snake.radius) * smooth;

                    let target_spacing = (agents[i].snake.radius * SNAKE_SPACING_MULT)
                        .max((agents[i].snake.radius * 0.78).max(5.2))
                        .min(SNAKE_SPACING_MAX);
                    agents[i].snake.segment_spacing =
                        agents[i].snake.segment_spacing + (target_spacing - agents[i].snake.segment_spacing) * smooth;
                }
                }
            }
        }

        if !net_mode && state != RunState::Finished {
            if pellets.total() < PELLET_TARGET_COUNT {
                pellets.refill_to(PELLET_TARGET_COUNT, PELLET_RADIUS);
            }

            if tokens.total() < TOKEN_TARGET_COUNT {
                tokens.refill_to_target();
            }
        }

        if let Some(net_agents) = net_agents.take() {
            agents = net_agents;
            state = RunState::Running;
            finish_reason = None;
            if !agents.is_empty() && !agents[0].alive {
                state = RunState::Spectating;
            }
        }

        for ev in runtime::drain_events() {
            match ev.kind.as_str() {
                "death" => {
                    if let Some(local_id) = runtime::local_player_id() {
                        if ev.id == local_id {
                            toast_text = "YOU DIED".to_owned();
                            toast_left = 1.5;
                        }
                    }
                }
                "time_add" => {
                    if !net_mode {
                        time_left += ev.id as f32;
                    }
                    time_add_flash = 0.9;
                    timeadd_badge_left = 1.4;
                }
                "magnet" => {
                    if let Some(local_id) = runtime::local_player_id() {
                        if ev.id == local_id {
                            net_magnet_left = TOKEN_DURATION_SEC;
                        }
                    }
                }
                "speedup" => {
                    if let Some(local_id) = runtime::local_player_id() {
                        if ev.id == local_id {
                            net_speedup_left = TOKEN_DURATION_SEC;
                        }
                    }
                }
                "time_up" => {
                    state = RunState::Finished;
                    finish_reason = Some(FinishReason::TimeUp);
                }
                _ => {}
            }
        }

        if net_mode {
            net_magnet_left = (net_magnet_left - dt).max(0.0);
            net_speedup_left = (net_speedup_left - dt).max(0.0);
        }

        {
            let segs = agents[0].snake.segments();
            let fit_px = screen_width().min(screen_height()) * CAMERA_FIT_SCREEN_FRACTION;

            if net_mode && !agents.is_empty() {
                camera_center = agents[0].snake.head_pos();
                let arena_extent_iso = ISO_SCALE * (2.0f32).sqrt() * ARENA_RADIUS;
                let target_extent = arena_extent_iso.max(1.0);
                let target_scale = (fit_px / target_extent).clamp(CAMERA_SCALE_MIN, CAMERA_SCALE_MAX);
                camera_scale = camera_scale + (target_scale - camera_scale) * (1.0 - (-6.0 * dt).exp());
            } else if state == RunState::Running && agents[0].alive {
                let mut center = vec2(0.0, 0.0);
                for p in segs {
                    center += *p;
                }
                if !segs.is_empty() {
                    center /= segs.len() as f32;
                }
                let cam_target = agents[0].snake.head_pos().lerp(center, CAMERA_CENTER_BODY_BLEND);
                let cam_smooth = 1.0 - (-10.0 * dt).exp();
                camera_center = camera_center.lerp(cam_target, cam_smooth);

                let body_len = ((agents[0].snake.target_length as f32) - 1.0).max(0.0) * agents[0].snake.segment_spacing;
                let approx_world_extent = (CAMERA_SNAKE_EXTENT_WORLD_MIN)
                    .max(body_len * CAMERA_SNAKE_EXTENT_WORLD_MULT);

                let target_extent_iso = approx_world_extent * ISO_SCALE * (2.0f32).sqrt();
                let extent_smooth = 1.0 - (-2.2 * dt).exp();
                camera_extent_smoothed = camera_extent_smoothed
                    + (target_extent_iso - camera_extent_smoothed) * extent_smooth;

                let target_scale = (fit_px / camera_extent_smoothed).clamp(CAMERA_SCALE_MIN, CAMERA_SCALE_MAX);
                let zoom_smooth = if target_scale < camera_scale {
                    1.0 - (-7.5 * dt).exp()
                } else {
                    1.0 - (-1.6 * dt).exp()
                };
                camera_scale = camera_scale + (target_scale - camera_scale) * zoom_smooth;
            } else {
                let arena_extent_iso = ISO_SCALE * (2.0f32).sqrt() * ARENA_RADIUS;
                let target_extent = arena_extent_iso.max(1.0);
                let extent_smooth = 1.0 - (-1.8 * dt).exp();
                camera_extent_smoothed = camera_extent_smoothed + (target_extent - camera_extent_smoothed) * extent_smooth;

                let base_scale = (fit_px / camera_extent_smoothed).max(0.0001);
                let target_scale = (base_scale * spectate_zoom).clamp(0.02, 6.0);
                let zoom_smooth = 1.0 - (-2.8 * dt).exp();
                camera_scale = camera_scale + (target_scale - camera_scale) * zoom_smooth;

                let pan_mult = if wants_boost { SPECTATE_PAN_BOOST_MULT } else { 1.0 };
                if desired_dir_world.length_squared() > 0.0001 {
                    camera_center += desired_dir_world.normalize() * (SPECTATE_PAN_SPEED * pan_mult) * dt;
                }

                let max_r = ARENA_RADIUS * SPECTATE_CAMERA_CLAMP_MULT;
                let d = camera_center.length();
                if d > max_r {
                    camera_center = camera_center / d * max_r;
                }
            }
        }

        set_default_camera();

        let corners = [
            vec2(0.0, 0.0),
            vec2(screen_width(), 0.0),
            vec2(0.0, screen_height()),
            vec2(screen_width(), screen_height()),
        ];
        let mut min_w = vec2(f32::INFINITY, f32::INFINITY);
        let mut max_w = vec2(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for c in corners {
            let w = screen_to_world(c, camera_center, screen_center, camera_scale);
            min_w.x = min_w.x.min(w.x);
            min_w.y = min_w.y.min(w.y);
            max_w.x = max_w.x.max(w.x);
            max_w.y = max_w.y.max(w.y);
        }

        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(10, 12, 18, 255));

        let w2s = |w: Vec2| world_to_screen(w, camera_center, screen_center, camera_scale);

        {
            let steps = 80;
            let mut prev: Option<Vec2> = None;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let a = t * std::f32::consts::TAU;
                let p = arena_center + vec2(a.cos(), a.sin()) * ARENA_RADIUS;
                let sp = w2s(p);
                if let Some(pp) = prev {
                    draw_line(pp.x, pp.y, sp.x, sp.y, 3.0, Color::from_rgba(255, 90, 90, 110));
                }
                prev = Some(sp);
            }
        }

        if !net_mode {
            pellets.draw_visible_aabb(min_w, max_w, w2s, 1.0);

            tokens.draw_visible_aabb(min_w, max_w, w2s);
        } else {
            for p in runtime::latest_pellets() {
                let sp = w2s(vec2(p.x, p.y));
                draw_circle(sp.x, sp.y, PELLET_RADIUS, Color::from_rgba(255, 220, 180, 220));
            }
            for t in runtime::latest_tokens() {
                if let Some(kind) = token_kind_from_str(&t.kind) {
                    let sp = w2s(vec2(t.pos.x, t.pos.y));
                    crate::game::food::draw_token_screen(sp, crate::game::food::token_radius(kind), kind);
                }
            }
        }

        for a in agents.iter() {
            if !a.alive {
                continue;
            }
            if net_mode {
                let base = a.color_head;
                let trail = runtime::trail_for(agent_id_from_name(&a.name));
                if trail.is_empty() {
                    let sp = w2s(a.snake.head_pos());
                    draw_circle(sp.x, sp.y, a.snake.radius * 1.25, Color::new(base.r, base.g, base.b, 0.18));
                    draw_circle(sp.x, sp.y, a.snake.radius, base);
                } else {
                    for (i, p) in trail.iter().enumerate() {
                        let sp = w2s(vec2(p.x, p.y));
                        let is_head = i == 0;
                        let col = if is_head { a.color_head } else { a.color_body };
                        let r = if is_head { a.snake.radius } else { a.snake.radius * 0.86 };
                        draw_circle(sp.x, sp.y, r, col);
                    }
                }
                continue;
            }
            let segs = a.snake.segments();
            let step = if heavy_mode {
                (segs.len() / 120).max(1)
            } else {
                1
            };
            for i in (0..segs.len()).rev() {
                if heavy_mode && i != 0 && (i % step != 0) {
                    continue;
                }
                let sp = w2s(segs[i]);
                let is_head = i == 0;
                let base = if is_head { a.color_head } else { a.color_body };
                draw_circle(sp.x, sp.y, a.snake.radius * 1.35, Color::new(base.r, base.g, base.b, 0.18));
                draw_circle(sp.x, sp.y, a.snake.radius, base);
            }
        }

        let ui_pad = 16.0 * ui_s;
        let left_x = ui_pad;
        let mut y = ui_pad + 26.0 * ui_s;

        draw_text("Snake Clash MVP", left_x, y, 28.0 * ui_s, WHITE);

        draw_text(
            &format!("FPS: {}", get_fps()),
            left_x,
            y + 22.0 * ui_s,
            18.0 * ui_s,
            Color::from_rgba(255, 255, 255, 140),
        );
        y += 30.0 * ui_s;

        let total_duration = MATCH_DURATION_SEC + time_added_total;
        let ext_text = if time_added_total > 0.0 {
            format!(" (+{}s)", time_added_total as i32)
        } else {
            String::new()
        };
        let time_color = if time_add_flash > 0.0 {
            Color::from_rgba(255, 180, 220, 240)
        } else {
            Color::from_rgba(255, 255, 255, 230)
        };
        draw_text(
            &format!("Time: {:05.1}s{}", time_left.max(0.0), ext_text),
            left_x,
            y,
            24.0 * ui_s,
            time_color,
        );
        y += 26.0 * ui_s;

        {
            let bar_x = left_x;
            let bar_y = y;
            let bar_w = 360.0 * ui_s;
            let bar_h = 10.0 * ui_s;
            let ratio = if total_duration > 0.0 {
                (time_left / total_duration).clamp(0.0, 1.0)
            } else {
                0.0
            };
            draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::from_rgba(255, 255, 255, 18));
            draw_rectangle(bar_x, bar_y, bar_w * ratio, bar_h, Color::from_rgba(255, 120, 200, 120));
            draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 1.0 * ui_s, Color::from_rgba(255, 255, 255, 40));
        }
        y += (10.0 + 18.0) * ui_s;

        if agents[0].alive {
            {
                let mut bx = left_x;
                let by = y;
                let magnet_left = if net_mode { net_magnet_left } else { agents[0].magnet_left };
                let speedup_left = if net_mode { net_speedup_left } else { agents[0].speedup_left };
                if magnet_left > 0.0 {
                    draw_token_badge(bx, by, TokenKind::Magnet, magnet_left);
                    bx += 140.0 * ui_s;
                }
                if speedup_left > 0.0 {
                    draw_token_badge(bx, by, TokenKind::SpeedUp, speedup_left);
                    bx += 140.0 * ui_s;
                }
                if timeadd_badge_left > 0.0 {
                    draw_token_badge(bx, by, TokenKind::TimeAdd, timeadd_badge_left);
                }

                if agents[0].magnet_left > 0.0 || agents[0].speedup_left > 0.0 || timeadd_badge_left > 0.0 {
                    y += 42.0 * ui_s;
                }
            }

            draw_text(
                &format!(
                    "Score: {}   Length: {}   Radius: {:.1}",
                    agents[0].score as i32,
                    agents[0].snake.target_length,
                    agents[0].snake.radius
                ),
                left_x,
                y,
                22.0 * ui_s,
                Color::from_rgba(255, 255, 255, 210),
            );
            y += 28.0 * ui_s;

            if !spectator_demo {
                let bar_x = left_x;
                let bar_y = y;
                let bar_w = 360.0 * ui_s;
                let bar_h = 14.0 * ui_s;
                let energy_ratio = (agents[0].boost_energy / ENERGY_BAR_MAX).clamp(0.0, 1.0);
                draw_rectangle_lines(
                    bar_x,
                    bar_y,
                    bar_w,
                    bar_h,
                    2.0 * ui_s,
                    Color::from_rgba(255, 255, 255, 90),
                );
                draw_rectangle(
                    bar_x,
                    bar_y,
                    bar_w * energy_ratio,
                    bar_h,
                    Color::from_rgba(90, 210, 255, 210),
                );
                draw_text("Boost", bar_x, bar_y - 6.0 * ui_s, 18.0 * ui_s, GRAY);
            }
        }

        if toast_left > 0.0 {
            let toast_x = left_x;
            let toast_y = y + 44.0 * ui_s;
            draw_rectangle(toast_x, toast_y - 22.0 * ui_s, 360.0 * ui_s, 28.0 * ui_s, Color::from_rgba(0, 0, 0, 70));
            draw_rectangle_lines(
                toast_x,
                toast_y - 22.0 * ui_s,
                360.0 * ui_s,
                28.0 * ui_s,
                2.0 * ui_s,
                Color::from_rgba(255, 255, 255, 35),
            );
            draw_text(&toast_text, toast_x + 10.0 * ui_s, toast_y, 22.0 * ui_s, Color::from_rgba(255, 255, 255, 235));
        }

        {
            let lb_w = 280.0 * ui_s;
            let lb_h = 200.0 * ui_s;
            let lb_x = screen_width() - lb_w - 16.0 * ui_s;
            let lb_y = 16.0 * ui_s;
            draw_rectangle(lb_x, lb_y, lb_w, lb_h, Color::from_rgba(0, 0, 0, 85));
            draw_rectangle_lines(lb_x, lb_y, lb_w, lb_h, 2.0 * ui_s, Color::from_rgba(255, 255, 255, 40));
            draw_text("LEADERBOARD", lb_x + 12.0 * ui_s, lb_y + 26.0 * ui_s, 20.0 * ui_s, WHITE);

            scratch.leaderboard_order.clear();
            scratch.leaderboard_order.extend(0..agents.len());
            scratch
                .leaderboard_order
                .sort_by(|&i, &j| agents[j].score.partial_cmp(&agents[i].score).unwrap());

            let mut row_y = lb_y + 56.0 * ui_s;
            for (rank, idx) in scratch.leaderboard_order.iter().copied().take(5).enumerate() {
                let a = &agents[idx];
                let name = if idx == 0 { "YOU" } else { &a.name };
                let color = if a.alive {
                    Color::from_rgba(255, 255, 255, 220)
                } else {
                    Color::from_rgba(255, 255, 255, 120)
                };
                draw_text(
                    &format!("{:>2}. {:<6} {:>6}", rank + 1, name, a.score as i32),
                    lb_x + 12.0 * ui_s,
                    row_y,
                    20.0 * ui_s,
                    color,
                );
                row_y += 24.0 * ui_s;
            }

            draw_text(
                &format!("BEST {:>6}", best_score),
                lb_x + 12.0 * ui_s,
                lb_y + lb_h - 12.0 * ui_s,
                18.0 * ui_s,
                Color::from_rgba(255, 255, 255, 160),
            );
        }

        {
            let mm_center = vec2(mm_x + mm_size * 0.5, mm_y + mm_size * 0.5);
            draw_rectangle(mm_x, mm_y, mm_size, mm_size, Color::from_rgba(0, 0, 0, 70));
            draw_rectangle_lines(mm_x, mm_y, mm_size, mm_size, 2.0 * ui_s, Color::from_rgba(255, 255, 255, 35));

            let r = mm_size * 0.46;
            draw_circle_lines(mm_center.x, mm_center.y, r, 2.0 * ui_s, Color::from_rgba(255, 90, 90, 120));

            {
                let mm_world_pos = if state == RunState::Spectating {
                    camera_center
                } else {
                    last_player_pos
                };
                let rel = (mm_world_pos - arena_center) / ARENA_RADIUS;
                let dot = mm_center + rel * r;
                draw_circle(dot.x, dot.y, 3.5 * ui_s, Color::from_rgba(255, 255, 255, 170));
            }

            for (idx, a) in agents.iter().enumerate() {
                if !a.alive {
                    continue;
                }
                let rel = (a.snake.head_pos() - arena_center) / ARENA_RADIUS;
                let dot = mm_center + rel * r;
                let col = if idx == 0 { YELLOW } else { a.color_head };
                draw_circle(dot.x, dot.y, 4.5 * ui_s, col);
            }
        }

        {
            draw_circle(joystick_origin.x, joystick_origin.y, joystick_radius, Color::from_rgba(255, 255, 255, 20));
            draw_circle_lines(joystick_origin.x, joystick_origin.y, joystick_radius, 2.0 * ui_s, Color::from_rgba(255, 255, 255, 40));
            let knob = joystick_origin + stick_dir_screen;
            draw_circle(knob.x, knob.y, 26.0 * ui_s, Color::from_rgba(255, 255, 255, 60));
        }

        {
            let c = if agents[0].alive && wants_boost && agents[0].boost_energy > 0.01 {
                Color::from_rgba(90, 210, 255, 65)
            } else {
                Color::from_rgba(255, 255, 255, 20)
            };
            draw_circle(boost_btn_center.x, boost_btn_center.y, boost_radius, c);
            draw_circle_lines(boost_btn_center.x, boost_btn_center.y, boost_radius, 2.0 * ui_s, Color::from_rgba(255, 255, 255, 40));
            draw_text("BOOST", boost_btn_center.x - 34.0 * ui_s, boost_btn_center.y + 7.0 * ui_s, 20.0 * ui_s, WHITE);
        }

        if state == RunState::Spectating {
            let r = zoom_btn_radius;

            let plus_fill = if zoom_plus_held {
                Color::from_rgba(90, 210, 255, 70)
            } else {
                Color::from_rgba(0, 0, 0, 80)
            };
            draw_circle(zoom_plus_center.x, zoom_plus_center.y, r, plus_fill);
            draw_circle_lines(
                zoom_plus_center.x,
                zoom_plus_center.y,
                r,
                2.0 * ui_s,
                Color::from_rgba(255, 255, 255, 40),
            );
            draw_text("+", zoom_plus_center.x - 10.0 * ui_s, zoom_plus_center.y + 12.0 * ui_s, 40.0 * ui_s, WHITE);

            let minus_fill = if zoom_minus_held {
                Color::from_rgba(90, 210, 255, 70)
            } else {
                Color::from_rgba(0, 0, 0, 80)
            };
            draw_circle(zoom_minus_center.x, zoom_minus_center.y, r, minus_fill);
            draw_circle_lines(
                zoom_minus_center.x,
                zoom_minus_center.y,
                r,
                2.0 * ui_s,
                Color::from_rgba(255, 255, 255, 40),
            );
            draw_text("-", zoom_minus_center.x - 8.0 * ui_s, zoom_minus_center.y + 12.0 * ui_s, 44.0 * ui_s, WHITE);
        }

        if state == RunState::Spectating && !spectator_demo {
            let w = 520.0 * ui_s;
            let h = 150.0 * ui_s;
            let x = (screen_width() - w) * 0.5;
            let y = screen_height() * 0.41;
            draw_rectangle(x, y, w, h, Color::from_rgba(0, 0, 0, 110));
            draw_rectangle_lines(x, y, w, h, 2.0 * ui_s, Color::from_rgba(255, 255, 255, 40));

            let title = "YOU DIED";
            let mt = measure_text(title, None, (56.0 * ui_s).round() as u16, 1.0);
            draw_text(title, x + (w - mt.width) * 0.5, y + 62.0 * ui_s, 56.0 * ui_s, RED);
            draw_text(
                &format!("SPECTATING  {:04.1}s", time_left.max(0.0)),
                x + 26.0 * ui_s,
                y + 108.0 * ui_s,
                28.0 * ui_s,
                Color::from_rgba(255, 255, 255, 220),
            );

            let hint = "Joystick: move • +/-: zoom";
            let mh = measure_text(hint, None, (20.0 * ui_s).round() as u16, 1.0);
            draw_text(
                hint,
                x + (w - mh.width) * 0.5,
                y + 134.0 * ui_s,
                20.0 * ui_s,
                Color::from_rgba(255, 255, 255, 170),
            );
        }

        if net_mode && countdown_left > 0.0 {
            let text = format!("MATCH STARTS IN {:02.0}", countdown_left.ceil());
            let mt = measure_text(&text, None, (48.0 * ui_s).round() as u16, 1.0);
            let cx = (screen_width() - mt.width) * 0.5;
            let cy = screen_height() * 0.22;
            draw_rectangle(
                cx - 20.0 * ui_s,
                cy - 44.0 * ui_s,
                mt.width + 40.0 * ui_s,
                60.0 * ui_s,
                Color::from_rgba(0, 0, 0, 120),
            );
            draw_rectangle_lines(
                cx - 20.0 * ui_s,
                cy - 44.0 * ui_s,
                mt.width + 40.0 * ui_s,
                60.0 * ui_s,
                2.0 * ui_s,
                Color::from_rgba(255, 255, 255, 50),
            );
            draw_text(&text, cx, cy, 48.0 * ui_s, Color::from_rgba(255, 255, 255, 240));
        }
        if state == RunState::Finished {
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));

            let (winner_idx, winner_score) = match finish_reason {
                Some(FinishReason::LastAlive) => {
                    let wi = finished_winner_idx.unwrap_or(0);
                    (wi, agents.get(wi).map(|a| a.score as i32).unwrap_or(0))
                }
                _ => agents
                    .iter()
                    .enumerate()
                    .fold((0usize, i32::MIN), |best, (i, a)| {
                        let s = a.score as i32;
                        if s > best.1 { (i, s) } else { best }
                    }),
            };
            let winner_name = agents.get(winner_idx).map(|a| a.name.as_str()).unwrap_or("?");

            let player_score = agents[0].score as i32;
            let player_is_winner = match finish_reason {
                Some(FinishReason::LastAlive) => winner_idx == 0,
                Some(FinishReason::AllEliminated) => false,
                _ => player_score >= winner_score,
            };

            match finish_reason {
                Some(FinishReason::AllEliminated) => {
                    let title = "ALL ELIMINATED";
                    let mt = measure_text(title, None, (56.0 * ui_s).round() as u16, 1.0);
                    draw_text(
                        title,
                        screen_width() * 0.5 - mt.width * 0.5,
                        screen_height() * 0.45,
                        56.0 * ui_s,
                        RED,
                    );
                }
                Some(FinishReason::LastAlive) => {
                    let title = if player_is_winner { "WINNER" } else { "LOSER" };
                    let mt = measure_text(title, None, (56.0 * ui_s).round() as u16, 1.0);
                    draw_text(
                        title,
                        screen_width() * 0.5 - mt.width * 0.5,
                        screen_height() * 0.45,
                        56.0 * ui_s,
                        WHITE,
                    );
                    let reason = "LAST SNAKE STANDING";
                    let mr = measure_text(reason, None, (20.0 * ui_s).round() as u16, 1.0);
                    draw_text(
                        reason,
                        screen_width() * 0.5 - mr.width * 0.5,
                        screen_height() * 0.48,
                        20.0 * ui_s,
                        Color::from_rgba(255, 255, 255, 170),
                    );
                }
                _ => {
                    let title = if player_is_winner { "WINNER" } else { "LOSER" };
                    let mt = measure_text(title, None, (56.0 * ui_s).round() as u16, 1.0);
                    draw_text(
                        title,
                        screen_width() * 0.5 - mt.width * 0.5,
                        screen_height() * 0.45,
                        56.0 * ui_s,
                        WHITE,
                    );
                }
            }

            draw_text(
                &format!("Your score: {}", player_score),
                screen_width() * 0.5 - 112.0 * ui_s,
                screen_height() * 0.50,
                28.0 * ui_s,
                WHITE,
            );
            draw_text(
                &format!("Winner: {}  ({})", winner_name, winner_score),
                screen_width() * 0.5 - 150.0 * ui_s,
                screen_height() * 0.54,
                24.0 * ui_s,
                Color::from_rgba(255, 255, 255, 200),
            );
            draw_text(
                &format!("Best score: {}", best_score),
                screen_width() * 0.5 - 106.0 * ui_s,
                screen_height() * 0.58,
                22.0 * ui_s,
                Color::from_rgba(255, 255, 255, 160),
            );
            draw_text(
                "Press R to restart",
                screen_width() * 0.5 - 120.0 * ui_s,
                screen_height() * 0.61,
                26.0 * ui_s,
                WHITE,
            );
        }

        if spectator_demo && state == RunState::Finished {
            demo_restart_left -= dt;
            if demo_restart_left <= 0.0 {
                agents = make_initial_agents();
                pellets.clear();
                pellets.populate_random(PELLET_TARGET_COUNT, PELLET_RADIUS);
                tokens.clear();
                tokens.populate_random();
                time_left = MATCH_DURATION_SEC;
                time_added_total = 0.0;
                time_add_flash = 0.0;
                best_score = 0;
                toast_left = 0.0;
                toast_text.clear();
                state = RunState::Spectating;
                finish_reason = None;
                finished_winner_idx = None;

                last_player_pos = agents[0].snake.head_pos();

                spectate_zoom = 1.0;
                prev_pinch_dist = None;

                camera_center = agents[0].snake.head_pos();
                camera_scale = CAMERA_SCALE_BASE;
                camera_extent_smoothed = 1.0;
            }
        }

        if is_key_pressed(KeyCode::R) && state == RunState::Finished {
            agents = make_initial_agents();
            pellets.clear();
            pellets.populate_random(PELLET_TARGET_COUNT, PELLET_RADIUS);
            tokens.clear();
            tokens.populate_random();
            time_left = MATCH_DURATION_SEC;
            time_added_total = 0.0;
            time_add_flash = 0.0;
            best_score = 0;
            toast_left = 0.0;
            toast_text.clear();
            state = if spectator_demo { RunState::Spectating } else { RunState::Running };
            finish_reason = None;
            finished_winner_idx = None;
            demo_restart_left = 0.0;

            last_player_pos = agents[0].snake.head_pos();

            spectate_zoom = 1.0;
            prev_pinch_dist = None;

            camera_center = agents[0].snake.head_pos();
            camera_scale = CAMERA_SCALE_BASE;
            camera_extent_smoothed = 1.0;
        }

        next_frame().await;
    }
}
