use macroquad::prelude::*;
use macroquad::rand::gen_range;

#[derive(Clone, Copy)]
pub struct Pellet {
    pub pos: Vec2,
    pub radius: f32,
    pub value: i32,
    pub color: Color,
}

pub struct Pellets {
    bucket_size: f32,
    world_radius: f32,
    min_cell: i32,
    dim: i32,
    buckets: Vec<Vec<Pellet>>,
    total: usize,
    scratch_reinsert: Vec<(usize, Pellet)>,
}

impl Pellets {
    pub fn new(bucket_size: f32, world_half_size: f32) -> Self {
        // Fixed grid (square) for fast lookups and iteration.
        // The world uses a circular arena, but a square grid is fine for bucketing.
        let half_cells = ((world_half_size / bucket_size).ceil() as i32) + 2;
        let dim = half_cells * 2 + 1;
        let bucket_count = (dim as usize) * (dim as usize);

        Self {
            bucket_size,
            world_radius: world_half_size,
            min_cell: -half_cells,
            dim,
            buckets: vec![Vec::new(); bucket_count],
            total: 0,
            scratch_reinsert: Vec::new(),
        }
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn clear(&mut self) {
        for b in &mut self.buckets {
            b.clear();
        }
        self.total = 0;
        self.scratch_reinsert.clear();
    }

    pub fn populate_random(&mut self, count: usize, radius: f32) {
        while self.total < count {
            let pellet = Self::random_pellet(self.world_radius, radius);
            self.insert(pellet);
        }
    }

    pub fn refill_to(&mut self, count: usize, radius: f32) {
        self.populate_random(count, radius);
    }

    pub fn spawn(&mut self, pos: Vec2, radius: f32, value: i32, color: Color) {
        let pellet = Pellet {
            pos,
            radius,
            value,
            color,
        };
        self.insert(pellet);
    }

    pub fn draw_visible_aabb<F>(&self, top_left: Vec2, bottom_right: Vec2, mut world_to_screen: F, radius_scale: f32)
    where
        F: FnMut(Vec2) -> Vec2,
    {
        let (min_cx0, min_cy0) = self.cell_of(top_left);
        let (max_cx0, max_cy0) = self.cell_of(bottom_right);
        let min_x = (min_cx0 - 1).clamp(self.min_cell, self.max_cell());
        let min_y = (min_cy0 - 1).clamp(self.min_cell, self.max_cell());
        let max_x = (max_cx0 + 1).clamp(self.min_cell, self.max_cell());
        let max_y = (max_cy0 + 1).clamp(self.min_cell, self.max_cell());

        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                let idx = self.bucket_index(cx, cy);
                let bucket = &self.buckets[idx];
                for p in bucket {
                    let sp = world_to_screen(p.pos);
                    draw_circle(sp.x, sp.y, p.radius * radius_scale, p.color);
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn spawn_corpse(&mut self, segments: &[Vec2], max_pellets: usize) {
        if segments.is_empty() {
            return;
        }

        let color = Color::from_rgba(255, 220, 120, 255);
        let count = segments.len().min(max_pellets).max(1);

        for k in 0..count {
            let idx = (k * segments.len()) / count;
            let p = segments[idx];
            let jx = ((k as f32 * 12.9898).sin() * 3.0).clamp(-3.0, 3.0);
            let jy = ((k as f32 * 78.233).cos() * 3.0).clamp(-3.0, 3.0);
            self.spawn(p + vec2(jx, jy), 4.2, 1, color);
        }
    }

    pub fn spawn_corpse_score(&mut self, segments: &[Vec2], total_value: i32, max_pellets: usize, spread_px: f32) {
        if segments.is_empty() || total_value <= 0 || max_pellets == 0 {
            return;
        }

        // We preserve total score value, but cap pellet count for perf.
        let count = (total_value as usize).min(max_pellets).max(1);
        let base = (total_value / count as i32).max(1);
        let remainder = (total_value - base * count as i32).max(0) as usize;

        // Slightly golden so it reads as a "corpse trail" drop.
        let base_color = Color::from_rgba(255, 220, 140, 255);

        for k in 0..count {
            let idx = (k * segments.len()) / count;
            let p = segments[idx];

            // Local tangent for a perpendicular jitter (so it follows the body shape).
            let prev = segments[idx.saturating_sub(1)];
            let next = segments[(idx + 1).min(segments.len() - 1)];
            let tan = next - prev;
            let tan_n = if tan.length_squared() > 0.0001 { tan.normalize() } else { vec2(1.0, 0.0) };
            let perp = vec2(-tan_n.y, tan_n.x);

            // Deterministic pseudo-jitter based on k (no RNG needed here)
            let fk = k as f32;
            let j1 = (fk * 12.9898).sin();
            let j2 = (fk * 78.233).cos();
            let off = perp * (j1 * spread_px) + tan_n * (j2 * spread_px * 0.35);

            let value = base + if k < remainder { 1 } else { 0 };

            // Visual: radius grows sublinearly with value, and is clamped.
            let v = value as f32;
            let radius = (3.4 + v.sqrt() * 1.0).clamp(3.4, 16.0);

            // Tiny tint shift with value so bigger drops pop a bit.
            let tint = (v / (v + 12.0)).clamp(0.0, 1.0);
            let color = Color::new(
                base_color.r * (0.92 + 0.08 * tint),
                base_color.g * (0.88 + 0.12 * tint),
                base_color.b * (0.78 + 0.22 * tint),
                1.0,
            );

            self.spawn(p + off, radius, value, color);
        }
    }

    pub fn eat_colliding(&mut self, head: Vec2, head_radius: f32, pickup_bonus: f32, max_eat: usize) -> i32 {
        if max_eat == 0 {
            return 0;
        }

        let reach = head_radius + 6.0 + pickup_bonus; // marge (+ magnet)
        let (min_cell_x, min_cell_y) = self.cell_of(head - vec2(reach, reach));
        let (max_cell_x, max_cell_y) = self.cell_of(head + vec2(reach, reach));

        let mut gained = 0;
        let mut eaten: usize = 0;

        for cy in min_cell_y..=max_cell_y {
            for cx in min_cell_x..=max_cell_x {
                if eaten >= max_eat {
                    return gained;
                }

                let idx = self.bucket_index(cx, cy);
                let bucket = &mut self.buckets[idx];

                let mut i = 0;
                while i < bucket.len() {
                    if eaten >= max_eat {
                        return gained;
                    }

                    let p = bucket[i];
                    let r = head_radius + pickup_bonus + p.radius;
                    if head.distance_squared(p.pos) <= r * r {
                        gained += p.value;
                        bucket.swap_remove(i);
                        self.total -= 1;
                        eaten += 1;
                        continue;
                    }
                    i += 1;
                }
            }
        }

        gained
    }

    pub fn apply_magnet(
        &mut self,
        head: Vec2,
        dt: f32,
        radius: f32,
        speed: f32,
        max_per_frame: usize,
    ) {
        if dt <= 0.0 || radius <= 0.0 || speed <= 0.0 || max_per_frame == 0 {
            return;
        }

        // Copy params locally so closures don't borrow `self` during bucket mutation.
        let bucket_size = self.bucket_size;
        let min_cell = self.min_cell;
        let max_cell = self.max_cell();
        let dim = self.dim;
        let buckets_len = self.buckets.len();

        let cell_of = |pos: Vec2| -> (i32, i32) {
            let cx = (pos.x / bucket_size).floor() as i32;
            let cy = (pos.y / bucket_size).floor() as i32;
            (cx.clamp(min_cell, max_cell), cy.clamp(min_cell, max_cell))
        };

        let bucket_index = |cx: i32, cy: i32| -> usize {
            let x = cx.clamp(min_cell, max_cell) - min_cell;
            let y = cy.clamp(min_cell, max_cell) - min_cell;
            let idx = (y * dim + x) as usize;
            idx.min(buckets_len.saturating_sub(1))
        };

        let r2 = radius * radius;
        let (min_cell_x, min_cell_y) = cell_of(head - vec2(radius, radius));
        let (max_cell_x, max_cell_y) = cell_of(head + vec2(radius, radius));

        let mut moved_count: usize = 0;
        self.scratch_reinsert.clear();

        let buckets = &mut self.buckets;
        let scratch_reinsert = &mut self.scratch_reinsert;

        'cells: for cy in min_cell_y..=max_cell_y {
            for cx in min_cell_x..=max_cell_x {
                if moved_count >= max_per_frame {
                    break 'cells;
                }

                let old_idx = bucket_index(cx, cy);
                let bucket = &mut buckets[old_idx];

                let mut i = 0;
                while i < bucket.len() {
                    if moved_count >= max_per_frame {
                        break 'cells;
                    }

                    let mut p = bucket[i];
                    let d2 = head.distance_squared(p.pos);
                    if d2 > r2 {
                        i += 1;
                        continue;
                    }

                    let d = d2.sqrt().max(0.0001);
                    let dir = (head - p.pos) / d;
                    let t = (1.0 - (d / radius)).clamp(0.0, 1.0);
                    let step = speed * dt * (0.20 + 0.80 * t);
                    p.pos += dir * step.min(d);

                    let (ncx, ncy) = cell_of(p.pos);
                    let new_idx = bucket_index(ncx, ncy);
                    if new_idx != old_idx {
                        bucket.swap_remove(i);
                        scratch_reinsert.push((new_idx, p));
                        moved_count += 1;
                        continue;
                    }

                    bucket[i] = p;
                    moved_count += 1;
                    i += 1;
                }
            }
        }

        for (idx, p) in scratch_reinsert.drain(..) {
            buckets[idx].push(p);
        }
    }

    pub fn best_pellet_target(&self, head: Vec2, search_radius: f32) -> Option<Vec2> {
        if search_radius <= 0.0 {
            return None;
        }

        let r2 = search_radius * search_radius;
        let (min_cell_x, min_cell_y) = self.cell_of(head - vec2(search_radius, search_radius));
        let (max_cell_x, max_cell_y) = self.cell_of(head + vec2(search_radius, search_radius));

        let mut best: Option<(Vec2, f32)> = None;

        for cy in min_cell_y..=max_cell_y {
            for cx in min_cell_x..=max_cell_x {
                let idx = self.bucket_index(cx, cy);
                let bucket = &self.buckets[idx];
                for p in bucket {
                    let d2 = head.distance_squared(p.pos);
                    if d2 > r2 {
                        continue;
                    }
                    let d = d2.sqrt().max(10.0);
                    // Prefer higher value pellets, but don't ignore distance.
                    let s = (p.value as f32) / d;
                    if best.map(|b| s > b.1).unwrap_or(true) {
                        best = Some((p.pos, s));
                    }
                }
            }
        }

        best.map(|b| b.0)
    }

    fn insert(&mut self, pellet: Pellet) {
        let (cx, cy) = self.cell_of(pellet.pos);
        let idx = self.bucket_index(cx, cy);
        self.buckets[idx].push(pellet);
        self.total += 1;
    }

    fn cell_of(&self, pos: Vec2) -> (i32, i32) {
        let cx = (pos.x / self.bucket_size).floor() as i32;
        let cy = (pos.y / self.bucket_size).floor() as i32;
        (
            cx.clamp(self.min_cell, self.max_cell()),
            cy.clamp(self.min_cell, self.max_cell()),
        )
    }

    fn max_cell(&self) -> i32 {
        self.min_cell + self.dim - 1
    }

    fn bucket_index(&self, cx: i32, cy: i32) -> usize {
        let x = cx.clamp(self.min_cell, self.max_cell()) - self.min_cell;
        let y = cy.clamp(self.min_cell, self.max_cell()) - self.min_cell;
        ((y * self.dim + x) as usize).min(self.buckets.len().saturating_sub(1))
    }

    fn random_pellet(world_half_size: f32, radius: f32) -> Pellet {
        // world_half_size est utilisé ici comme un "rayon" de spawn (arène circulaire)
        let a = gen_range(0.0f32, std::f32::consts::TAU);
        let r = gen_range(0.0f32, 1.0f32).sqrt() * world_half_size;
        let x = a.cos() * r;
        let y = a.sin() * r;

        // Petites variations de valeur/couleur pour un rendu "snake.io-like"
        let roll = gen_range(0, 100);
        let (value, color, r) = if roll < 72 {
            (1, Color::from_rgba(120, 220, 255, 255), radius)
        } else if roll < 94 {
            (2, Color::from_rgba(170, 255, 130, 255), radius * 1.45)
        } else {
            (5, Color::from_rgba(255, 120, 200, 255), radius * 2.25)
        };

        Pellet {
            pos: vec2(x, y),
            radius: r,
            value,
            color,
        }
    }
}
