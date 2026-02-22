use std::sync::{Mutex, OnceLock};

use macroquad::prelude::*;

use crate::client::master_api::{create_room, delete_room, fetch_rooms, join_room, RoomInfo};
use crate::client::runtime;

static MENU_STATE: OnceLock<Mutex<MenuState>> = OnceLock::new();

struct MenuState {
    in_game: bool,
    rooms: Vec<RoomInfo>,
    selected: usize,
    player_name: String,
    last_refresh: f32,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            in_game: false,
            rooms: fetch_rooms(),
            selected: 0,
            player_name: "PLAYER".to_owned(),
            last_refresh: 0.0,
        }
    }
}

pub fn update() -> bool {
    let state = MENU_STATE.get_or_init(|| Mutex::new(MenuState::default()));
    let mut guard = match state.lock() {
        Ok(g) => g,
        Err(_) => return true,
    };

    if guard.in_game {
        return false;
    }

    let dt = get_frame_time();
    guard.last_refresh += dt;

    clear_background(Color::from_rgba(10, 12, 18, 255));
    let w = screen_width();
    let h = screen_height();
    let ui_s = 1.0;

    draw_text("Snake Clash MVP", 32.0, 56.0, 36.0, WHITE);
    draw_text("MENU", 32.0, 88.0, 22.0, Color::from_rgba(255, 255, 255, 180));

    // Room list panel
    let panel_x = 32.0;
    let panel_y = 120.0;
    let panel_w = w - 64.0;
    let panel_h = h - 240.0;
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::from_rgba(0, 0, 0, 90));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, Color::from_rgba(255, 255, 255, 40));

    let y = panel_y + 36.0;
    let mut join_target: Option<usize> = None;
    for (idx, room) in guard.rooms.iter().enumerate() {
        let row_h = 38.0;
        let row_y = y + (idx as f32) * (row_h + 8.0);
        let is_sel = idx == guard.selected;
        let bg = if is_sel { Color::from_rgba(90, 210, 255, 40) } else { Color::from_rgba(0, 0, 0, 0) };
        draw_rectangle(panel_x + 16.0, row_y - 22.0, panel_w - 32.0, row_h, bg);
        draw_text(
            &format!("{}  {} ({}/{})", room.room_id, room.name, room.players, room.max_players),
            panel_x + 24.0,
            row_y,
            20.0 * ui_s,
            WHITE,
        );
        if button_hit(panel_x + panel_w - 140.0, row_y - 22.0, 100.0, row_h, "JOIN") {
            join_target = Some(idx);
        }
    }

    if let Some(idx) = join_target {
        let room = guard.rooms.get(idx).cloned();
        if let Some(room) = room {
            guard.selected = idx;
            if let Some(server_addr) = join_room(&room.room_id, &guard.player_name, None) {
                runtime::init(server_addr);
                    runtime::send_join(room.room_id.clone(), guard.player_name.clone(), "desktop".to_owned());
                guard.in_game = true;
            }
        }
    }

    if button_hit(panel_x + 16.0, panel_y + panel_h - 56.0, 140.0, 36.0, "REFRESH") {
        guard.rooms = fetch_rooms();
        guard.selected = 0;
        guard.last_refresh = 0.0;
    }

    if button_hit(panel_x + 172.0, panel_y + panel_h - 56.0, 140.0, 36.0, "CREATE") {
        let room_name = format!("{}'s Room", guard.player_name);
        if let Some(created) = create_room(&room_name, 4) {
            guard.rooms = fetch_rooms();
            if let Some(pos) = guard.rooms.iter().position(|r| r.room_id == created.room_id) {
                guard.selected = pos;
            }
            guard.last_refresh = 0.0;
        }
    }

    if button_hit(panel_x + 328.0, panel_y + panel_h - 56.0, 140.0, 36.0, "DELETE") {
        if let Some(room) = guard.rooms.get(guard.selected) {
            if delete_room(&room.room_id) {
                guard.rooms = fetch_rooms();
                if guard.rooms.is_empty() {
                    guard.selected = 0;
                } else {
                    guard.selected = guard.selected.min(guard.rooms.len() - 1);
                }
                guard.last_refresh = 0.0;
            }
        }
    }

    draw_text(
        &format!("Last refresh: {:04.1}s", guard.last_refresh),
        panel_x + 16.0,
        panel_y + panel_h - 86.0,
        18.0,
        Color::from_rgba(255, 255, 255, 140),
    );

    true
}

fn button_hit(x: f32, y: f32, w: f32, h: f32, label: &str) -> bool {
    let hovered = {
        let (mx, my) = mouse_position();
        mx >= x && mx <= x + w && my >= y && my <= y + h
    };
    let pressed = hovered && is_mouse_button_pressed(MouseButton::Left);
    let col = if hovered { Color::from_rgba(90, 210, 255, 70) } else { Color::from_rgba(0, 0, 0, 60) };
    draw_rectangle(x, y, w, h, col);
    draw_rectangle_lines(x, y, w, h, 2.0, Color::from_rgba(255, 255, 255, 50));
    draw_text(label, x + 16.0, y + h * 0.7, 20.0, WHITE);
    pressed
}
