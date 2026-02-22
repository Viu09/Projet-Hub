#[derive(Debug, Default)]
pub struct WorldState {
    pub server_tick: u32,
}

impl WorldState {
    pub fn step(&mut self) {
        self.server_tick = self.server_tick.saturating_add(1);
    }
}
