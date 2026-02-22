// DTO model (requested by client).
#[derive(Clone)]
#[allow(dead_code)]
pub struct Snake {
    pub body: Vec<(f32, f32)>,
    pub direction: (f32, f32),
    pub speed: f32,
    pub alive: bool,
}
