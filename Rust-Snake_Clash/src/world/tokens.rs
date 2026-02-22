use macroquad::prelude::*;
use macroquad::rand::gen_range;

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
            // Guarantee at least one of each (especially TimeAdd so it is visible in gameplay)
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
            // Higher is better
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

    fn random_pos(&self) -> Vec2 {
        // Uniform in disk
        let a = gen_range(0.0f32, std::f32::consts::TAU);
        let r = gen_range(0.0f32, 1.0f32).sqrt() * self.arena_radius;
        vec2(a.cos() * r, a.sin() * r)
    }

    fn random_token(&self) -> Token {
        // Weighted: TimeAdd should be noticeable in gameplay
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

    // simple icon-like shapes (no sprites)
    match kind {
        TokenKind::Magnet => {
            // diamond
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
            // triangle arrow
            let p = screen_pos;
            let r = radius;
            let pts = [vec2(p.x - r * 0.6, p.y - r), vec2(p.x + r, p.y), vec2(p.x - r * 0.6, p.y + r)];
            draw_triangle(pts[0], pts[1], pts[2], fill);
            draw_line(pts[0].x, pts[0].y, pts[1].x, pts[1].y, 2.0, stroke);
            draw_line(pts[1].x, pts[1].y, pts[2].x, pts[2].y, 2.0, stroke);
            draw_line(pts[2].x, pts[2].y, pts[0].x, pts[0].y, 2.0, stroke);
        }
        TokenKind::TimeAdd => {
            // plus sign inside a circle (so it's not confused with pink pellets)
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
