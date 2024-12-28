use super::super::channel::{APUChannel, TimedAPUChannel};
use super::super::envelope::{EnvelopeGenerator, EnvelopedChannel};
use super::super::sequencer::Sequencer;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Sweeper {
    enabled: bool,
    divider_period_reload_value: u8,
    divider_period_counter: u8,
    negative: bool,
    shift_count: u8,

    reload_flag: bool,

    target_period: u16,

    is_square_1: bool,
}

impl Sweeper {
    fn new(is_square_1: bool) -> Self {
        Self {
            enabled: false,
            divider_period_reload_value: 0,
            divider_period_counter: 0,
            negative: false,
            shift_count: 0,

            reload_flag: false,

            target_period: 0,

            is_square_1,
        }
    }

    fn update_target_period(&mut self, pulse_period: u16) {
        let change_amount = pulse_period >> self.shift_count;

        self.target_period = if self.negative {
            pulse_period
                .saturating_sub(change_amount)
                .saturating_sub(self.is_square_1 as u16)
        } else {
            pulse_period.saturating_add(change_amount)
        };
    }
}

const DUTY_CYCLE_SEQUENCES: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

#[derive(Serialize, Deserialize)]
pub struct SquarePulse {
    period: u16,
    current_timer: u16,

    envelope_generator: EnvelopeGenerator,
    sequencer: Sequencer,
    sweeper: Sweeper,
}

impl SquarePulse {
    pub fn new(is_square_1: bool) -> Self {
        Self {
            period: 0,
            current_timer: 0,

            envelope_generator: EnvelopeGenerator::new(),
            sequencer: Sequencer::new(),
            sweeper: Sweeper::new(is_square_1),
        }
    }

    pub(crate) fn set_duty_cycle_index(&mut self, duty_cycle_index: u8) {
        self.sequencer
            .set_sequence(&DUTY_CYCLE_SEQUENCES[duty_cycle_index as usize & 0x3]);
    }

    pub(crate) fn get_period(&self) -> u16 {
        self.period
    }

    pub(crate) fn set_period(&mut self, period: u16) {
        self.period = period;

        self.sweeper.update_target_period(self.period);
    }

    pub(crate) fn set_sweeper_data(&mut self, data: u8) {
        self.sweeper.enabled = data & 0x80 != 0;
        self.sweeper.divider_period_reload_value = (data >> 4) & 0b111;
        self.sweeper.negative = data & 0x08 != 0;
        self.sweeper.shift_count = data & 0b111;

        self.sweeper.reload_flag = true;

        self.sweeper.update_target_period(self.period);
    }

    pub(crate) fn clock_sweeper(&mut self) {
        if self.sweeper.divider_period_counter == 0 && self.sweeper.enabled && !self.muted() {
            self.set_period(self.sweeper.target_period);
        }

        if self.sweeper.divider_period_counter == 0 || self.sweeper.reload_flag {
            self.sweeper.divider_period_counter = self.sweeper.divider_period_reload_value;
            self.sweeper.reload_flag = false;
        } else {
            self.sweeper.divider_period_counter -= 1;
        }
    }

    pub(crate) fn muted(&self) -> bool {
        self.period < 8 || (!self.sweeper.negative && self.sweeper.target_period > 0x7FF)
    }

    pub(crate) fn reset(&mut self) {
        self.sequencer.reset();
    }
}

impl EnvelopedChannel for SquarePulse {
    fn clock_envlope(&mut self) {
        self.envelope_generator.clock();
    }

    fn envelope_generator_mut(&mut self) -> &mut EnvelopeGenerator {
        &mut self.envelope_generator
    }
}

impl APUChannel for SquarePulse {
    fn get_output(&mut self) -> f32 {
        if self.muted() || self.sequencer.get_current_value() == 0 {
            0.
        } else {
            self.envelope_generator.get_current_volume()
        }
    }
}

impl TimedAPUChannel for SquarePulse {
    fn timer_clock(&mut self) {
        if self.current_timer == 0 {
            self.sequencer.clock();

            self.current_timer = self.period;
        } else {
            self.current_timer -= 1;
        }
    }
}
