use std::collections::VecDeque;

use macroquad::prelude::*;

pub struct SnakeSim {
    pub head: Vec2,
    dir: Vec2,

    pub speed: f32,
    pub turn_rate: f32,

    pub segment_spacing: f32,
    pub radius: f32,
    pub target_length: usize,

    trail: VecDeque<Vec2>,
    segments: Vec<Vec2>,
}

impl SnakeSim {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::new_at(vec2(0.0, 0.0), vec2(1.0, 0.0))
    }

    pub fn new_at(head: Vec2, dir: Vec2) -> Self {
        let dir = if dir.length_squared() > 0.0001 {
            dir.normalize()
        } else {
            vec2(1.0, 0.0)
        };

        let target_length = crate::config::BASE_SNAKE_LENGTH;
        let segment_spacing = crate::config::BASE_SNAKE_SPACING;
        let radius = crate::config::BASE_SNAKE_RADIUS;

        let mut trail = VecDeque::new();
        trail.push_front(head);

        let segments = vec![head; target_length];

        Self {
            head,
            dir,
            speed: crate::config::BASE_SPEED,
            turn_rate: 10.0,
            segment_spacing,
            radius,
            target_length,
            trail,
            segments,
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    #[allow(dead_code)]
    pub fn reset_at(&mut self, head: Vec2, dir: Vec2) {
        *self = Self::new_at(head, dir);
    }

    pub fn head_pos(&self) -> Vec2 {
        self.head
    }

    pub fn dir(&self) -> Vec2 {
        self.dir
    }

    #[allow(dead_code)]
    pub fn tail_pos(&self) -> Vec2 {
        *self.segments.last().unwrap_or(&self.head)
    }

    pub fn segments(&self) -> &[Vec2] {
        &self.segments
    }

    pub fn update_dir(&mut self, dt: f32, desired_dir_world: Vec2) {
        if desired_dir_world.length_squared() > 0.0001 {
            let desired_dir = desired_dir_world.normalize();
            let t = (self.turn_rate * dt).clamp(0.0, 1.0);
            let new_dir = self.dir.lerp(desired_dir, t);
            if new_dir.length_squared() > 0.0001 {
                self.dir = new_dir.normalize();
            }
        }

        self.head += self.dir * self.speed * dt;

        let min_sample = crate::config::TRAIL_SAMPLE_MIN_DIST;
        let should_push = self
            .trail
            .front()
            .map(|p| p.distance(self.head) >= min_sample)
            .unwrap_or(true);

        if should_push {
            self.trail.push_front(self.head);
        } else if let Some(front) = self.trail.front_mut() {
            *front = self.head;
        }

        let max_needed = (self.target_length as f32 * self.segment_spacing) + self.segment_spacing;
        self.trim_trail(max_needed);

        self.rebuild_segments();
    }

    #[allow(dead_code)]
    pub fn update(&mut self, dt: f32, mouse_world: Vec2) {
        let desired = mouse_world - self.head;
        let dir = if desired.length_squared() > 1.0 {
            desired.normalize()
        } else {
            vec2(0.0, 0.0)
        };
        self.update_dir(dt, dir);
    }

    fn trim_trail(&mut self, max_len: f32) {
        let mut acc = 0.0;
        for i in 0..self.trail.len().saturating_sub(1) {
            let a = self.trail[i];
            let b = self.trail[i + 1];
            acc += a.distance(b);
            if acc > max_len {
                let keep = i + 2;
                while self.trail.len() > keep {
                    self.trail.pop_back();
                }
                return;
            }
        }
    }

    fn rebuild_segments(&mut self) {
        if self.segments.len() != self.target_length {
            self.segments.resize(self.target_length, self.head);
        }

        for i in 0..self.target_length {
            let dist = i as f32 * self.segment_spacing;
            self.segments[i] = self.sample_trail(dist);
        }
    }

    fn sample_trail(&self, distance_from_head: f32) -> Vec2 {
        if self.trail.len() == 1 {
            return self.head;
        }

        let mut remaining = distance_from_head;
        for i in 0..self.trail.len() - 1 {
            let a = self.trail[i];
            let b = self.trail[i + 1];
            let seg_len = a.distance(b);
            if seg_len <= 0.0001 {
                continue;
            }

            if remaining <= seg_len {
                let t = remaining / seg_len;
                return a.lerp(b, t);
            }

            remaining -= seg_len;
        }

        *self.trail.back().unwrap_or(&self.head)
    }
}
