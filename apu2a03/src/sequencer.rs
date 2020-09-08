use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Sequencer {
    sequence: Vec<u8>,
    position: usize,
}

impl Sequencer {
    pub(crate) fn new() -> Self {
        Self {
            sequence: Vec::new(),
            position: 0,
        }
    }

    pub(crate) fn set_sequence(&mut self, sequence: &[u8]) {
        self.sequence.clear();

        self.sequence.extend_from_slice(sequence);
    }

    pub(crate) fn get_current_value(&self) -> u8 {
        *self.sequence.get(self.position).unwrap_or(&0)
    }

    fn length(&self) -> usize {
        self.sequence.len()
    }

    pub(crate) fn clock(&mut self) {
        if self.length() != 0 {
            self.position = (self.position.wrapping_add(1)) % self.length();
        }
    }

    pub(crate) fn reset(&mut self) {
        self.position = 0;
    }
}
