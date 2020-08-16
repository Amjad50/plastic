use crate::channels::{Dmc, NoiseWave, SquarePulse, TriangleWave};
use crate::length_counter::LengthCountedChannel;
use crate::tone_source::APUChannel;
use std::sync::{Arc, Mutex};

pub struct Mixer {
    square_pulse_1: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_2: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    triangle: Arc<Mutex<LengthCountedChannel<TriangleWave>>>,
    noise: Arc<Mutex<LengthCountedChannel<NoiseWave>>>,
    dmc: Arc<Mutex<Dmc>>,
}

impl Mixer {
    pub fn new(
        square_pulse_1: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
        square_pulse_2: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
        triangle: Arc<Mutex<LengthCountedChannel<TriangleWave>>>,
        noise: Arc<Mutex<LengthCountedChannel<NoiseWave>>>,
        dmc: Arc<Mutex<Dmc>>,
    ) -> Self {
        Self {
            square_pulse_1,
            square_pulse_2,
            triangle,
            noise,
            dmc,
        }
    }

    fn channel_output<C: APUChannel>(channel: &mut Arc<Mutex<C>>) -> f32 {
        if let Ok(mut channel) = channel.lock() {
            channel.get_output()
        } else {
            0.
        }
    }
}

impl APUChannel for Mixer {
    fn get_output(&mut self) -> f32 {
        let square_pulse_1 = Self::channel_output(&mut self.square_pulse_1);
        let square_pulse_2 = Self::channel_output(&mut self.square_pulse_2);
        let triangle = Self::channel_output(&mut self.triangle);
        let noise = Self::channel_output(&mut self.noise);
        let dmc = Self::channel_output(&mut self.dmc);

        let pulse_out = if square_pulse_1 == 0. && square_pulse_2 == 0. {
            0.
        } else {
            95.88 / ((8128. / (square_pulse_1 + square_pulse_2)) + 100.)
        };

        let tnd_out = if triangle == 0. && noise == 0. && dmc == 0. {
            0.
        } else {
            159.79 / ((1. / ((triangle / 8227.) + (noise / 12241.) + (dmc / 22638.))) + 100.)
        };

        pulse_out + tnd_out
    }
}
