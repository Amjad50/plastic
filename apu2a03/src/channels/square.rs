use crate::envelope::{EnvelopeGenerator, EnvelopedChannel};
use crate::sequencer::Sequencer;
use crate::tone_source::APUChannel;

const DUTY_CYCLE_SEQUENCES: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

pub struct SquarePulse {
    period: u16,
    current_timer: u16,

    envelope_generator: EnvelopeGenerator,
    sequencer: Sequencer,

    muted: bool,
}

impl SquarePulse {
    pub fn new() -> Self {
        Self {
            period: 0,
            current_timer: 0,

            envelope_generator: EnvelopeGenerator::new(),
            sequencer: Sequencer::new(),

            muted: false,
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

        self.muted = self.period > 0x7FF || self.period < 8;
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
        if self.muted || self.sequencer.get_current_value() == 0 {
            0.
        } else {
            self.envelope_generator.get_current_volume()
        }
    }

    fn timer_clock(&mut self) {
        if self.current_timer == 0 {
            self.sequencer.clock();

            self.current_timer = self.period;
        } else {
            self.current_timer -= 1;
        }
    }
}
