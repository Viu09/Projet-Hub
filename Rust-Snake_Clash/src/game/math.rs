use macroquad::prelude::*;

use crate::config::ISO_SCALE;

pub fn iso_project(rel_world: Vec2) -> Vec2 {
    vec2((rel_world.x - rel_world.y) * ISO_SCALE, (rel_world.x + rel_world.y) * ISO_SCALE)
}

pub fn iso_unproject(rel_iso: Vec2) -> Vec2 {
    let u = rel_iso.x / ISO_SCALE; // x - y
    let v = rel_iso.y / ISO_SCALE; // x + y
    vec2((u + v) * 0.5, (v - u) * 0.5)
}

pub fn world_to_screen(world: Vec2, camera_center: Vec2, screen_center: Vec2, camera_scale: f32) -> Vec2 {
    let rel = world - camera_center;
    let iso = iso_project(rel);
    screen_center + iso * camera_scale
}

pub fn screen_to_world(screen: Vec2, camera_center: Vec2, screen_center: Vec2, camera_scale: f32) -> Vec2 {
    let iso = (screen - screen_center) / camera_scale;
    camera_center + iso_unproject(iso)
}

pub fn screen_vec_to_world_dir(screen_vec: Vec2, camera_scale: f32) -> Vec2 {
    let world_vec = iso_unproject(screen_vec / camera_scale);
    if world_vec.length_squared() > 0.0001 {
        world_vec.normalize()
    } else {
        vec2(0.0, 0.0)
    }
}

pub fn point_in_circle(p: Vec2, center: Vec2, radius: f32) -> bool {
    p.distance_squared(center) <= radius * radius
}

pub fn ui_anchor_portrait(pos_720x1280: (f32, f32)) -> Vec2 {
    let x_ref = pos_720x1280.0;
    let y_ref = pos_720x1280.1;

    let x = if x_ref <= 360.0 {
        x_ref
    } else {
        screen_width() - (720.0 - x_ref)
    };

    vec2(x, screen_height() - (1280.0 - y_ref))
}

pub fn input_pos_scale() -> f32 {
    #[cfg(target_arch = "wasm32")]
    {
        screen_dpi_scale()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        1.0
    }
}
