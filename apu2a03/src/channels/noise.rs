use crate::tone_source::APUChannel;

/// Table for NTSC only
const NOISE_PERIODS_TABLE: [u16; 0x10] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub struct NoiseWave {
    freq: f32,
    n_harmonics: u8,
    sample_num: usize,

    period: u16,
    current_timer: u16,
    // TODO: add a method to stay in sync with one reference stored in APU
    reference_frequency: f32,

    shift_register: u16,

    mode_flag: bool,
}

impl NoiseWave {
    pub fn new(reference_frequency: f32) -> Self {
        Self {
            freq: 0.,
            n_harmonics: 20, // default
            sample_num: 0,

            period: 0,
            current_timer: 0,
            reference_frequency,

            shift_register: 1,

            mode_flag: false,
        }
    }

    pub(crate) fn get_period(&self) -> u16 {
        self.period
    }

    pub(crate) fn set_period(&mut self, period_index_index: u8) {
        self.period = NOISE_PERIODS_TABLE[period_index_index as usize & 0xF];

        self.update_frequency();
    }

    pub(crate) fn set_mode_flag(&mut self, flag: bool) {
        self.mode_flag = flag;
    }

    pub(crate) fn reset(&mut self) {
        self.sample_num = 0;
    }

    pub(crate) fn clock_timer(&mut self) {
        if self.current_timer == 0 {
            let selected_bit_location = if self.mode_flag { 6 } else { 1 };
            let bit_0 = self.shift_register & 1;
            let selected_bit = (self.shift_register >> selected_bit_location) & 1;
            let feedback = bit_0 ^ selected_bit;

            self.shift_register = (self.shift_register >> 1) & 0x3FFF;
            self.shift_register |= feedback << 14;

            self.current_timer = self.period + 1;
        } else {
            self.current_timer = self.current_timer.saturating_sub(1);
        }
    }

    fn update_frequency(&mut self) {
        self.freq = self.reference_frequency / (1 * (self.period + 1)) as f32;
    }

    fn generate_next(&mut self, time: usize) -> f32 {
        1.
    }

    fn muted(&self) -> bool {
        self.shift_register & 1 != 0
    }
}

impl Iterator for NoiseWave {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.muted() {
            Some(0.)
        } else {
            self.sample_num = self.sample_num.wrapping_add(1);

            let result = self.generate_next(self.sample_num);

            Some(result)
        }
    }
}

impl APUChannel for NoiseWave {}
