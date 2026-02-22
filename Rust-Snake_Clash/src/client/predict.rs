#![allow(dead_code)]

#[derive(Default)]
pub struct InputBuffer {
    pub seq: u32,
}

impl InputBuffer {
    pub fn next_seq(&mut self) -> u32 {
        self.seq = self.seq.saturating_add(1);
        self.seq
    }
}
