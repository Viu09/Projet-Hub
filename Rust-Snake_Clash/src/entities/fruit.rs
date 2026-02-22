use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::constants::{CELL_SIZE, GRID_HEIGHT, GRID_WIDTH};
use crate::entities::snake::Snake;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FruitKind {
    Apple,
    Peach,
    Cherry,
}

#[derive(Clone, Copy, Debug)]
pub struct Fruit {
    pub x: i32,
    pub y: i32,
    pub kind: FruitKind,
}

impl FruitKind {
    pub fn points(self) -> i32 {
        match self {
            FruitKind::Apple => 1,
            FruitKind::Peach => 2,  // 2x
            FruitKind::Cherry => 5, // 5x
        }
    }

    pub fn color(self) -> Color {
        match self {
            FruitKind::Apple => RED,
            FruitKind::Peach => ORANGE,
            FruitKind::Cherry => PINK,
        }
    }
}

impl Fruit {
    fn is_far_enough_from_fruits(pos: (i32, i32), fruits: &[Fruit], min_dist_px: f32) -> bool {
        let (x, y) = pos;
        let min_dist_sq = min_dist_px * min_dist_px;

        for f in fruits {
            let dx = (x - f.x) as f32 * CELL_SIZE;
            let dy = (y - f.y) as f32 * CELL_SIZE;
            if dx * dx + dy * dy < min_dist_sq {
                return false;
            }
        }
        true
    }

    pub fn new_random_with_constraints(
        snake: &Snake,
        existing_fruits: &[Fruit],
        min_dist_px: f32,
        kind: FruitKind,
    ) -> Option<Self> {
        const MAX_ATTEMPTS: usize = 20_000;

        for _ in 0..MAX_ATTEMPTS {
            let x = gen_range(0, GRID_WIDTH);
            let y = gen_range(0, GRID_HEIGHT);
            let pos = (x, y);

            if snake.is_collision(pos) {
                continue;
            }
            if existing_fruits.iter().any(|f| f.x == x && f.y == y) {
                continue;
            }
            if !Self::is_far_enough_from_fruits(pos, existing_fruits, min_dist_px) {
                continue;
            }

            return Some(Self { x, y, kind });
        }

        None
    }

    pub fn draw(&self, offset_x: f32, offset_y: f32) {
        let px = offset_x + self.x as f32 * CELL_SIZE + CELL_SIZE / 2.0;
        let py = offset_y + self.y as f32 * CELL_SIZE + CELL_SIZE / 2.0;
        draw_circle(px, py, CELL_SIZE / 2.5, self.kind.color());
    }
}