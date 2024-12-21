use super::super::channel::{APUChannel, TimedAPUChannel};
use super::super::envelope::{EnvelopeGenerator, EnvelopedChannel};
use serde::{Deserialize, Serialize};

/// Table for NTSC only
const NOISE_PERIODS_TABLE: [u16; 0x10] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Serialize, Deserialize)]
pub struct NoiseWave {
    period: u16,
    current_timer: u16,

    envelope_generator: EnvelopeGenerator,

    shift_register: u16,

    mode_flag: bool,
}

impl NoiseWave {
    pub fn new() -> Self {
        Self {
            period: 0,
            current_timer: 0,

            envelope_generator: EnvelopeGenerator::new(),

            shift_register: 1,

            mode_flag: false,
        }
    }

    pub(crate) fn set_period(&mut self, period_index_index: u8) {
        self.period = NOISE_PERIODS_TABLE[period_index_index as usize & 0xF];
    }

    pub(crate) fn set_mode_flag(&mut self, flag: bool) {
        self.mode_flag = flag;
    }
}

impl APUChannel for NoiseWave {
    fn get_output(&mut self) -> f32 {
        if self.shift_register & 1 == 0 {
            0.
        } else {
            self.envelope_generator.get_current_volume()
        }
    }
}

impl TimedAPUChannel for NoiseWave {
    fn timer_clock(&mut self) {
        if self.current_timer == 0 {
            let selected_bit_location = if self.mode_flag { 6 } else { 1 };
            let bit_0 = self.shift_register & 1;
            let selected_bit = (self.shift_register >> selected_bit_location) & 1;
            let feedback = bit_0 ^ selected_bit;

            self.shift_register = (self.shift_register >> 1) & 0x3FFF;
            self.shift_register |= feedback << 14;

            self.current_timer = self.period;
        } else {
            self.current_timer = self.current_timer.saturating_sub(1);
        }
    }
}

impl EnvelopedChannel for NoiseWave {
    fn clock_envlope(&mut self) {
        self.envelope_generator.clock()
    }

    fn envelope_generator_mut(&mut self) -> &mut EnvelopeGenerator {
        &mut self.envelope_generator
    }
}
