use macroquad::prelude::*;
use macroquad::rand::gen_range;

// ---- Pellets ----

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

    pub fn positions(&self) -> Vec<Vec2> {
        let mut out = Vec::with_capacity(self.total);
        for bucket in &self.buckets {
            for p in bucket {
                out.push(p.pos);
            }
        }
        out
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

        let count = (total_value as usize).min(max_pellets).max(1);
        let base = (total_value / count as i32).max(1);
        let remainder = (total_value - base * count as i32).max(0) as usize;

        let base_color = Color::from_rgba(255, 220, 140, 255);

        for k in 0..count {
            let idx = (k * segments.len()) / count;
            let p = segments[idx];

            let prev = segments[idx.saturating_sub(1)];
            let next = segments[(idx + 1).min(segments.len() - 1)];
            let tan = next - prev;
            let tan_n = if tan.length_squared() > 0.0001 { tan.normalize() } else { vec2(1.0, 0.0) };
            let perp = vec2(-tan_n.y, tan_n.x);

            let fk = k as f32;
            let j1 = (fk * 12.9898).sin();
            let j2 = (fk * 78.233).cos();
            let off = perp * (j1 * spread_px) + tan_n * (j2 * spread_px * 0.35);

            let value = base + if k < remainder { 1 } else { 0 };

            let v = value as f32;
            let radius = (3.4 + v.sqrt() * 1.0).clamp(3.4, 16.0);

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

        let reach = head_radius + 6.0 + pickup_bonus;
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
        let a = gen_range(0.0f32, std::f32::consts::TAU);
        let r = gen_range(0.0f32, 1.0f32).sqrt() * world_half_size;
        let x = a.cos() * r;
        let y = a.sin() * r;

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

// ---- Tokens ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Magnet,
    SpeedUp,
    TimeAdd,
}

#[derive(Clone, Copy, Debug)]
pub struct Token {
    pub pos: Vec2,
    pub kind: TokenKind,
}

pub struct Tokens {
    arena_radius: f32,
    target_count: usize,
    items: Vec<Token>,
}

impl Tokens {
    pub fn new(arena_radius: f32, target_count: usize) -> Self {
        Self {
            arena_radius,
            target_count,
            items: Vec::new(),
        }
    }

    pub fn total(&self) -> usize {
        self.items.len()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn populate_random(&mut self) {
        while self.items.len() < self.target_count {
            let (m, s, t) = self.count_kinds();
            let kind = if t == 0 {
                TokenKind::TimeAdd
            } else if m == 0 {
                TokenKind::Magnet
            } else if s == 0 {
                TokenKind::SpeedUp
            } else {
                self.random_token().kind
            };
            self.items.push(Token { pos: self.random_pos(), kind });
        }
    }

    pub fn refill_to_target(&mut self) {
        self.populate_random();
    }

    #[allow(dead_code)]
    pub fn collect_colliding(&mut self, head: Vec2, head_radius: f32) -> Vec<TokenKind> {
        self.collect_colliding_filtered(head, head_radius, |_| true)
    }

    pub fn collect_colliding_filtered<F>(&mut self, head: Vec2, head_radius: f32, mut allow: F) -> Vec<TokenKind>
    where
        F: FnMut(TokenKind) -> bool,
    {
        let mut collected: Vec<TokenKind> = Vec::new();

        let mut i = 0;
        while i < self.items.len() {
            let t = self.items[i];
            let tr = token_radius(t.kind);
            let r = head_radius + tr;
            if head.distance_squared(t.pos) <= r * r {
                if allow(t.kind) {
                    collected.push(t.kind);
                    self.items.swap_remove(i);
                    continue;
                }
            }
            i += 1;
        }

        collected
    }

    pub fn best_target<F>(&self, head: Vec2, mut score: F) -> Option<(Vec2, TokenKind)>
    where
        F: FnMut(TokenKind) -> Option<f32>,
    {
        let mut best: Option<(Vec2, TokenKind, f32)> = None;
        for t in &self.items {
            let Some(w) = score(t.kind) else {
                continue;
            };

            let d = head.distance(t.pos).max(1.0);
            let s = w / d;
            if best.map(|b| s > b.2).unwrap_or(true) {
                best = Some((t.pos, t.kind, s));
            }
        }
        best.map(|(p, k, _)| (p, k))
    }

    pub fn draw_visible_aabb<F>(&self, top_left: Vec2, bottom_right: Vec2, mut world_to_screen: F)
    where
        F: FnMut(Vec2) -> Vec2,
    {
        for t in &self.items {
            if t.pos.x < top_left.x
                || t.pos.x > bottom_right.x
                || t.pos.y < top_left.y
                || t.pos.y > bottom_right.y
            {
                continue;
            }

            let sp = world_to_screen(t.pos);
            let r = token_radius(t.kind);
            draw_token_screen(sp, r, t.kind);
        }
    }

    pub fn items(&self) -> &[Token] {
        &self.items
    }

    fn random_pos(&self) -> Vec2 {
        let a = gen_range(0.0f32, std::f32::consts::TAU);
        let r = gen_range(0.0f32, 1.0f32).sqrt() * self.arena_radius;
        vec2(a.cos() * r, a.sin() * r)
    }

    fn random_token(&self) -> Token {
        let roll = gen_range(0, 100);
        let kind = if roll < 40 {
            TokenKind::Magnet
        } else if roll < 80 {
            TokenKind::SpeedUp
        } else {
            TokenKind::TimeAdd
        };
        Token {
            pos: self.random_pos(),
            kind,
        }
    }

    fn count_kinds(&self) -> (usize, usize, usize) {
        let mut m = 0;
        let mut s = 0;
        let mut t = 0;
        for it in &self.items {
            match it.kind {
                TokenKind::Magnet => m += 1,
                TokenKind::SpeedUp => s += 1,
                TokenKind::TimeAdd => t += 1,
            }
        }
        (m, s, t)
    }
}

pub fn token_radius(kind: TokenKind) -> f32 {
    match kind {
        TokenKind::Magnet => 19.0,
        TokenKind::SpeedUp => 19.0,
        TokenKind::TimeAdd => 21.0,
    }
}

pub fn draw_token_screen(screen_pos: Vec2, radius: f32, kind: TokenKind) {
    let (fill, stroke) = match kind {
        TokenKind::Magnet => (Color::from_rgba(120, 220, 255, 200), Color::from_rgba(120, 220, 255, 255)),
        TokenKind::SpeedUp => (Color::from_rgba(170, 255, 130, 200), Color::from_rgba(170, 255, 130, 255)),
        TokenKind::TimeAdd => (Color::from_rgba(255, 120, 200, 200), Color::from_rgba(255, 120, 200, 255)),
    };

    match kind {
        TokenKind::Magnet => {
            let p = screen_pos;
            let r = radius;
            let pts = [
                vec2(p.x, p.y - r),
                vec2(p.x + r, p.y),
                vec2(p.x, p.y + r),
                vec2(p.x - r, p.y),
            ];
            draw_triangle(pts[0], pts[1], pts[2], fill);
            draw_triangle(pts[2], pts[3], pts[0], fill);
            for i in 0..4 {
                let a = pts[i];
                let b = pts[(i + 1) % 4];
                draw_line(a.x, a.y, b.x, b.y, 2.0, stroke);
            }
        }
        TokenKind::SpeedUp => {
            let p = screen_pos;
            let r = radius;
            let pts = [vec2(p.x - r * 0.6, p.y - r), vec2(p.x + r, p.y), vec2(p.x - r * 0.6, p.y + r)];
            draw_triangle(pts[0], pts[1], pts[2], fill);
            draw_line(pts[0].x, pts[0].y, pts[1].x, pts[1].y, 2.0, stroke);
            draw_line(pts[1].x, pts[1].y, pts[2].x, pts[2].y, 2.0, stroke);
            draw_line(pts[2].x, pts[2].y, pts[0].x, pts[0].y, 2.0, stroke);
        }
        TokenKind::TimeAdd => {
            let p = screen_pos;
            let r = radius;
            draw_circle(p.x, p.y, r, Color::from_rgba(255, 120, 200, 70));
            draw_circle_lines(p.x, p.y, r, 2.0, stroke);
            draw_rectangle(p.x - r * 0.2, p.y - r, r * 0.4, r * 2.0, fill);
            draw_rectangle(p.x - r, p.y - r * 0.2, r * 2.0, r * 0.4, fill);
            draw_rectangle_lines(p.x - r * 0.2, p.y - r, r * 0.4, r * 2.0, 2.0, stroke);
            draw_rectangle_lines(p.x - r, p.y - r * 0.2, r * 2.0, r * 0.4, 2.0, stroke);
        }
    }
}
