use crate::sequencer::Sequencer;
use crate::tone_source::{APUChannel, TimedAPUChannel};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TriangleWave {
    period: u16,
    current_timer: u16,

    sequencer: Sequencer,

    muted: bool,

    linear_counter_reload_value: u8,
    linear_counter: u8,
    linear_counter_control_flag: bool,
    linear_counter_reload_flag: bool,
}

impl TriangleWave {
    pub fn new() -> Self {
        let mut sequencer = Sequencer::new();
        sequencer.set_sequence(&[
            15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
            11, 12, 13, 14, 15,
        ]);

        Self {
            period: 0,
            current_timer: 0,

            sequencer,

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
    }

    pub(crate) fn set_linear_counter_reload_value(&mut self, value: u8) {
        self.linear_counter_reload_value = value;
    }

    pub(crate) fn set_linear_counter_control_flag(&mut self, flag: bool) {
        self.linear_counter_control_flag = flag;
    }

    pub(crate) fn set_linear_counter_reload_flag(&mut self, flag: bool) {
        self.linear_counter_reload_flag = flag;
    }

    pub(crate) fn clock_linear_counter(&mut self) {
        if self.linear_counter_reload_flag {
            // clear if control flag is also clear
            self.linear_counter_reload_flag = self.linear_counter_control_flag;

            self.linear_counter = self.linear_counter_reload_value;

            // clear only if the control flag is cleared
            if !self.linear_counter_control_flag {
                self.linear_counter_reload_flag = false;
            }
        } else {
            if self.linear_counter != 0 {
                self.linear_counter = self.linear_counter.saturating_sub(1);
            }
        }
    }
}

impl APUChannel for TriangleWave {
    fn get_output(&mut self) -> f32 {
        if self.linear_counter == 0 || self.muted {
            0.
        } else {
            self.sequencer.get_current_value() as f32
        }
    }
}

impl TimedAPUChannel for TriangleWave {
    fn timer_clock(&mut self) {
        if self.current_timer == 0 {
            self.sequencer.clock();

            self.current_timer = self.period;
        } else {
            self.current_timer -= 1;
        }
    }
}
