use crate::apu2a03_registers::Register;
use crate::channels::SquarePulse;
use crate::tone_source::APUChannelPlayer;
use std::sync::{Arc, Mutex};

pub struct APU2A03 {
    square_pulse_1: Arc<Mutex<SquarePulse>>,
    square_pulse_1_timer: u16,
    square_pulse_2: Arc<Mutex<SquarePulse>>,
    square_pulse_2_timer: u16,

    reference_clock_frequency: f32,
}

impl APU2A03 {
    pub fn new() -> Self {
        Self {
            square_pulse_1: Arc::new(Mutex::new(SquarePulse::new(440., 0.5, 20))),
            square_pulse_1_timer: 0,
            square_pulse_2: Arc::new(Mutex::new(SquarePulse::new(440., 0.5, 20))),
            square_pulse_2_timer: 0,

            reference_clock_frequency: 1.789773 * 1E6,
        }
    }

    pub fn get_square_pulse_1_player(&self) -> APUChannelPlayer<SquarePulse> {
        APUChannelPlayer::from_clone(self.square_pulse_1.clone())
    }

    pub fn get_square_pulse_2_player(&self) -> APUChannelPlayer<SquarePulse> {
        APUChannelPlayer::from_clone(self.square_pulse_2.clone())
    }

    pub fn update_reference_frequency(&mut self, freq: f32) {
        self.reference_clock_frequency = freq;
    }

    pub(crate) fn read_register(&self, register: Register) -> u8 {
        match register {
            Register::Status => 0,
            _ => {
                // unreadable
                0
            }
        }
    }

    pub(crate) fn write_register(&mut self, register: Register, data: u8) {
        match register {
            Register::Pulse1_1 => {
                let duty_cycle = [0.125, 0.25, 0.5, 0.75][data as usize >> 6];
                let volume = data & 0xF;
                let use_volume = data & 0x10 != 0;

                if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
                    square_pulse_1.set_duty_cycle(duty_cycle);
                    square_pulse_1.set_volume(volume, use_volume);
                }
            }
            Register::Pulse1_2 => {
                // sweep
            }
            Register::Pulse1_3 => {
                // low timer bits
                self.square_pulse_1_timer = (self.square_pulse_1_timer & 0xFF00) | data as u16;

                self.update_sqaure_pulse_1_frequency();
            }
            Register::Pulse1_4 => {
                // high timer bits
                self.square_pulse_1_timer =
                    (self.square_pulse_1_timer & 0xFF) | ((data as u16 & 0b111) << 8);

                self.update_sqaure_pulse_1_frequency();
            }
            Register::Pulse2_1 => {
                let duty_cycle = [0.125, 0.25, 0.5, 0.75][data as usize >> 6];
                let volume = data & 0xF;
                let use_volume = data & 0x10 != 0;

                if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
                    square_pulse_2.set_duty_cycle(duty_cycle);
                    square_pulse_2.set_volume(volume, use_volume);
                }
            }
            Register::Pulse2_2 => {
                // sweep
            }
            Register::Pulse2_3 => {
                // low timer bits
                self.square_pulse_2_timer = (self.square_pulse_2_timer & 0xFF00) | data as u16;

                self.update_sqaure_pulse_2_frequency();
            }
            Register::Pulse2_4 => {
                // high timer bits
                self.square_pulse_2_timer =
                    (self.square_pulse_2_timer & 0xFF) | ((data as u16 & 0b111) << 8);

                self.update_sqaure_pulse_2_frequency();
            }
            Register::Triangle_1 => {}
            Register::Triangle_2 => {}
            Register::Triangle_3 => {}
            Register::Triangle_4 => {}
            Register::Noise_1 => {}
            Register::Noise_2 => {}
            Register::Noise_3 => {}
            Register::Noise_4 => {}
            Register::DMC_1 => {}
            Register::DMC_2 => {}
            Register::DMC_3 => {}
            Register::DMC_4 => {}
            Register::Status => {
                // enable and disable channels

                self.square_pulse_1
                    .lock()
                    .unwrap()
                    .set_enable((data >> 0 & 1) != 0);
                self.square_pulse_2
                    .lock()
                    .unwrap()
                    .set_enable((data >> 1 & 1) != 0);
            }
            Register::FrameCounter => {}
        }
    }

    fn update_sqaure_pulse_1_frequency(&mut self) {
        if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
            square_pulse_1.set_freq(
                self.reference_clock_frequency / (16 * (self.square_pulse_1_timer + 1)) as f32,
            );
        }
    }

    fn update_sqaure_pulse_2_frequency(&mut self) {
        if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
            square_pulse_2.set_freq(
                self.reference_clock_frequency / (16 * (self.square_pulse_2_timer + 1)) as f32,
            );
        }
    }

    pub fn play(&self) {
        let device = rodio::default_output_device().unwrap();
        let sink = rodio::Sink::new(&device);

        let sqr1 = self.get_square_pulse_1_player();
        let sqr2 = self.get_square_pulse_2_player();

        let (controller, mixer) = rodio::dynamic_mixer::mixer::<f32>(5, crate::SAMPLE_RATE);

        controller.add(sqr1);
        controller.add(sqr2);

        sink.append(mixer);
        sink.set_volume(0.01);

        sink.play();
        sink.detach();
    }

    pub fn clock(&mut self) {}
}
