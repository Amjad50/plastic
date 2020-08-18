use crate::channels::{Dmc, NoiseWave, SquarePulse, TriangleWave};
use crate::length_counter::LengthCountedChannel;
use crate::tone_source::APUChannel;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Mixer {
    square_pulse_1: Rc<RefCell<LengthCountedChannel<SquarePulse>>>,
    square_pulse_2: Rc<RefCell<LengthCountedChannel<SquarePulse>>>,
    triangle: Rc<RefCell<LengthCountedChannel<TriangleWave>>>,
    noise: Rc<RefCell<LengthCountedChannel<NoiseWave>>>,
    dmc: Rc<RefCell<Dmc>>,
}

impl Mixer {
    pub fn new(
        square_pulse_1: Rc<RefCell<LengthCountedChannel<SquarePulse>>>,
        square_pulse_2: Rc<RefCell<LengthCountedChannel<SquarePulse>>>,
        triangle: Rc<RefCell<LengthCountedChannel<TriangleWave>>>,
        noise: Rc<RefCell<LengthCountedChannel<NoiseWave>>>,
        dmc: Rc<RefCell<Dmc>>,
    ) -> Self {
        Self {
            square_pulse_1,
            square_pulse_2,
            triangle,
            noise,
            dmc,
        }
    }
}

impl APUChannel for Mixer {
    fn get_output(&mut self) -> f32 {
        let square_pulse_1 = self.square_pulse_1.borrow_mut().get_output();
        let square_pulse_2 = self.square_pulse_2.borrow_mut().get_output();
        let triangle = self.triangle.borrow_mut().get_output();
        let noise = self.noise.borrow_mut().get_output();
        let dmc = self.dmc.borrow_mut().get_output();

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
