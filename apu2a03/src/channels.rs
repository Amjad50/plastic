use crate::tone_source::APUChannel;

pub struct SquarePulse {
    freq: f32,
    duty_cycle: f32,
    n_harmonics: u8,
    volume: u8,
    use_volume: bool,
    sample_num: usize,
}

impl SquarePulse {
    pub fn new(freq: f32, duty_cycle: f32, n_harmonics: u8) -> Self {
        Self {
            freq,
            duty_cycle,
            n_harmonics,
            volume: 0,
            use_volume: true,
            sample_num: 0,
        }
    }

    pub(crate) fn set_freq(&mut self, freq: f32) {
        self.freq = freq;
    }

    pub(crate) fn set_volume(&mut self, vol: u8, use_vol: bool) {
        assert!(vol < 0x10);

        self.volume = vol;
        self.use_volume = use_vol;
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
        self.sample_num = self.sample_num.wrapping_add(1);

        let result = self.sin_next(self.sample_num);

        if self.use_volume {
            Some(self.volume as f32 / 0xF as f32 * result)
        } else {
            Some(result)
        }
    }
}

impl APUChannel for SquarePulse {}
