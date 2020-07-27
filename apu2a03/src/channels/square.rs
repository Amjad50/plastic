use crate::tone_source::APUChannel;

pub struct SquarePulse {
    freq: f32,
    duty_cycle: f32,
    n_harmonics: u8,
    sample_num: usize,

    period: u16,
    // TODO: add a method to stay in sync with one reference stored in APU
    reference_frequency: f32,

    muted: bool,
}

impl SquarePulse {
    pub fn new(freq: f32, duty_cycle: f32, n_harmonics: u8, reference_frequency: f32) -> Self {
        Self {
            freq,
            duty_cycle,
            n_harmonics,
            sample_num: 0,

            period: 0,
            reference_frequency,

            muted: false,
        }
    }

    pub(crate) fn set_duty_cycle(&mut self, duty_cycle: f32) {
        // FIXME: very ineffecient, since it will mostly not happen, but we still
        // check 2 times
        self.duty_cycle = if duty_cycle < 0. {
            0.
        } else if duty_cycle > 1. {
            1.
        } else {
            duty_cycle
        };
    }

    pub(crate) fn get_period(&self) -> u16 {
        self.period
    }

    pub(crate) fn set_period(&mut self, period: u16) {
        self.period = period;

        self.update_frequency();
    }

    pub(crate) fn reset(&mut self) {
        self.sample_num = 0;
    }

    fn update_frequency(&mut self) {
        self.muted = self.period > 0x7FF || self.period < 8;
        self.freq = self.reference_frequency / (16 * (self.period + 1)) as f32;
    }

    /// returns a square function using sum of sines
    fn sin_next(&self, time: usize) -> f32 {
        let mut a: f32 = 0.;
        let mut b: f32 = 0.;
        let p = self.duty_cycle * 2. * 3.1415;

        for i in 1..=self.n_harmonics {
            let n = i as f32;
            let c = n * self.freq / self.sample_rate() as f32 * time as f32 * 2. * 3.1415;
            a += c.sin() / n;
            b += (c - p * n).sin() / n;
        }

        (a - b) / 2.
    }
}

impl Iterator for SquarePulse {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.muted {
            Some(0.)
        } else {
            self.sample_num = self.sample_num.wrapping_add(1);

            let result = self.sin_next(self.sample_num);

            Some(result)
        }
    }
}

impl APUChannel for SquarePulse {}
