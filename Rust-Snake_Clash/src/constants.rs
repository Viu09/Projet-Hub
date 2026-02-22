// Legacy (ancienne version grille/pommes). Gardé pour référence.
#[allow(dead_code)]
pub const CELL_SIZE: f32 = 20.0;
#[allow(dead_code)]
pub const GRID_WIDTH: i32 = 50;
#[allow(dead_code)]
pub const GRID_HEIGHT: i32 = 50;

#[allow(dead_code)]
pub const APPLE_START_COUNT: usize = 5;
#[allow(dead_code)]
pub const APPLE_MAX_COUNT: usize = 10;
#[allow(dead_code)]
pub const APPLE_MIN_DIST_PX: f32 = 80.0;

// Point 2 (snake.io-like): monde + pellets
#[allow(dead_code)]
pub const WORLD_HALF_SIZE: f32 = 4000.0;
#[cfg(any(feature = "demo100", feature = "demo_play100"))]
pub const PELLET_TARGET_COUNT: usize = 1800;
#[cfg(not(any(feature = "demo100", feature = "demo_play100")))]
pub const PELLET_TARGET_COUNT: usize = 4000;
pub const PELLET_BUCKET_SIZE: f32 = 140.0;
pub const PELLET_RADIUS: f32 = 4.0;

// MVP Snake Clash (solo)
pub const MATCH_DURATION_SEC: f32 = 90.0;

// Arène circulaire (mort immédiate si sortie)
pub const ARENA_RADIUS: f32 = 2600.0;

pub const BASE_SNAKE_LENGTH: usize = 18;
// Nombre de points requis pour gagner 1 segment de plus.
pub const SCORE_PER_SEGMENT: i32 = 12;

// Growth (SnakeClash-like): length + slight body size increase with score
pub const BASE_SNAKE_RADIUS: f32 = 6.0;
pub const MAX_SNAKE_RADIUS: f32 = 38.0;
// Smooth asymptotic growth: t = score / (score + half)
pub const SNAKE_RADIUS_SCORE_HALF: f32 = 160.0;
pub const SNAKE_RADIUS_GROWTH_EXP: f32 = 1.15;
pub const BASE_SNAKE_SPACING: f32 = 7.0;
pub const SNAKE_SPACING_MULT: f32 = 0.92;
pub const SNAKE_SPACING_MAX: f32 = 26.0;

// Trail sampling: keep turn detail even for big snakes
pub const TRAIL_SAMPLE_MIN_DIST: f32 = 2.0;

// Boost (énergie qui regen, conforme PDF)
pub const BASE_SPEED: f32 = 220.0;
pub const BOOST_SPEED_MULT: f32 = 1.55;
pub const BOOST_ENERGY_MAX: f32 = 100.0;
pub const BOOST_ENERGY_DRAIN_PER_SEC: f32 = 55.0;
pub const BOOST_ENERGY_REGEN_PER_SEC: f32 = 32.0;

// Movement tuning: small snakes should feel slower (screen-speed wise)
pub const SMALL_SNAKE_SPEED_MULT: f32 = 0.72;

// Tokens (PDF)
#[cfg(any(feature = "demo100", feature = "demo_play100"))]
pub const TOKEN_TARGET_COUNT: usize = 8;
#[cfg(not(any(feature = "demo100", feature = "demo_play100")))]
pub const TOKEN_TARGET_COUNT: usize = 12;
pub const TOKEN_DURATION_SEC: f32 = 10.0;
pub const TOKEN_TIME_ADD_SEC: f32 = 10.0;
pub const MAGNET_PICKUP_BONUS_PX: f32 = 28.0;
pub const SPEEDUP_MULT: f32 = 1.50;

// Demo (client): lots of bots, aim for smoothness
pub const DEMO_BOT_COUNT: usize = 100;

// Magnet feel (attraction)
pub const MAGNET_ATTRACT_RADIUS: f32 = 260.0;
pub const MAGNET_ATTRACT_SPEED: f32 = 520.0;
pub const MAGNET_ATTRACT_MAX_PER_FRAME: usize = 260;

// Safety: cap pellet consumes per frame (prevents magnet from "vacuuming" the world instantly)
pub const PELLET_EAT_MAX_PER_FRAME: usize = 10;

// Death / corpse drop: convert your score to pellets along the body shape
// Keep pellet count bounded for perf while preserving total value.
#[cfg(any(feature = "demo100", feature = "demo_play100"))]
pub const CORPSE_DROP_MAX_PELLETS: usize = 180;
#[cfg(not(any(feature = "demo100", feature = "demo_play100")))]
pub const CORPSE_DROP_MAX_PELLETS: usize = 650;
pub const CORPSE_DROP_SPREAD_PX: f32 = 10.0;

// Visual tuning (2D isométrique)
pub const ISO_SCALE: f32 = 0.90;

// Camera zoom: bigger snake => zoom out a bit (keep more body visible)
pub const CAMERA_SCALE_BASE: f32 = 1.0;
pub const CAMERA_SCALE_MIN: f32 = 0.42;
pub const CAMERA_SCALE_MAX: f32 = 1.0;
pub const CAMERA_FIT_SCREEN_FRACTION: f32 = 0.44;
pub const CAMERA_CENTER_BODY_BLEND: f32 = 0.35;

// Camera stability: use a shape-independent extent estimate so zoom doesn't "pump"
// when the snake coils/straightens.
pub const CAMERA_SNAKE_EXTENT_WORLD_MULT: f32 = 0.55;
pub const CAMERA_SNAKE_EXTENT_WORLD_MIN: f32 = 240.0;

// Spectator zoom
pub const SPECTATE_ZOOM_MIN: f32 = 0.30;
pub const SPECTATE_ZOOM_MAX: f32 = 3.25;
pub const SPECTATE_ZOOM_WHEEL_SENS: f32 = 0.09;

// Spectator (after death): free camera pan with joystick
pub const SPECTATE_PAN_SPEED: f32 = 720.0;
pub const SPECTATE_PAN_BOOST_MULT: f32 = 2.2;
pub const SPECTATE_CAMERA_CLAMP_MULT: f32 = 1.25;

// UI / contrôles (portrait)
pub const UI_SCALE: f32 = 0.80;
pub const UI_JOYSTICK_CENTER: (f32, f32) = (560.0, 1040.0);
pub const UI_JOYSTICK_RADIUS: f32 = 90.0;
pub const UI_JOYSTICK_DEADZONE: f32 = 0.12;
pub const UI_BOOST_BUTTON_CENTER: (f32, f32) = (140.0, 1040.0);
pub const UI_BOOST_BUTTON_RADIUS: f32 = 70.0;

// Spectator zoom buttons (visible when dead)
pub const UI_ZOOM_PLUS_CENTER: (f32, f32) = (640.0, 560.0);
pub const UI_ZOOM_MINUS_CENTER: (f32, f32) = (640.0, 660.0);
pub const UI_ZOOM_BUTTON_RADIUS: f32 = 42.0;

// UI
pub const ENERGY_BAR_MAX: f32 = BOOST_ENERGY_MAX;
