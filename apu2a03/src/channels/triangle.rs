use crate::tone_source::APUChannel;

pub struct TriangleWave {
    freq: f32,
    n_harmonics: u8,
    sample_num: usize,

    period: u16,
    // TODO: add a method to stay in sync with one reference stored in APU
    reference_frequency: f32,

    muted: bool,

    linear_counter_reload_value: u8,
    linear_counter: u8,
    linear_counter_control_flag: bool,
    linear_counter_reload_flag: bool,
}

impl TriangleWave {
    pub fn new(freq: f32, n_harmonics: u8, reference_frequency: f32) -> Self {
        Self {
            freq,
            n_harmonics,
            sample_num: 0,

            period: 0,
            reference_frequency,

            muted: false,

            linear_counter_reload_value: 0,
            linear_counter: 0,
            linear_counter_control_flag: false,
            linear_counter_reload_flag: false,
        }
    }

    pub(crate) fn get_period(&self) -> u16 {
        self.period
    }

    pub(crate) fn set_period(&mut self, period: u16) {
        self.period = period;

        self.muted = period < 2;

        self.update_frequency();
    }

    pub(crate) fn set_linear_counter_reload_value(&mut self, value: u8) {
        self.linear_counter_reload_value = value;
    }

    pub(crate) fn set_linear_counter_control_flag(&mut self, flag: bool) {
        self.linear_counter_control_flag = flag;

        if !flag {
            self.linear_counter_reload_flag = false;
        }
    }

    pub(crate) fn set_linear_counter_reload_flag(&mut self, flag: bool) {
        self.linear_counter_reload_flag = flag;
    }

    pub(crate) fn clock_linear_counter(&mut self) {
        if self.linear_counter_reload_flag {
            // clear if control flag is also clear
            self.linear_counter_reload_flag = self.linear_counter_control_flag;

            self.linear_counter = self.linear_counter_reload_value;
        } else {
            if self.linear_counter != 0 {
                self.linear_counter = self.linear_counter.saturating_sub(1);
            }
        }
    }

    fn update_frequency(&mut self) {
        self.freq = self.reference_frequency / (32 * (self.period + 1)) as f32;
    }

    /// returns a triangle function using sum of sines
    fn sin_next(&self, time: usize) -> f32 {
        let mut a: f32 = 0.;

        for i in 0..=(self.n_harmonics - 1) {
            // v = (-1)^i
            let v = (-1. as f32).powi(i as i32);
            let n = (2 * i as u32 + 1) as f32;
            let c = n * self.freq / self.sample_rate() as f32 * time as f32 * 2. * 3.1415;
            a += v * n.powi(-2) * c.sin();
        }

        a * (8. / (3.1415 as f32).powi(2))
    }
}

impl Iterator for TriangleWave {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.linear_counter == 0 || self.muted {
            Some(0.)
        } else {
            self.sample_num = self.sample_num.wrapping_add(1);

            let result = self.sin_next(self.sample_num);

            Some(result)
        }
    }
}

impl APUChannel for TriangleWave {}
