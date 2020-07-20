use crate::apu2a03_registers::Register;
use crate::channels::SquarePulse;
use crate::length_counter::LengthCountedChannel;
use crate::tone_source::APUChannelPlayer;
use std::sync::{Arc, Mutex};

pub struct APU2A03 {
    square_pulse_1: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_1_timer: u16,
    square_pulse_2: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_2_timer: u16,

    reference_clock_frequency: f32,

    is_4_step_squence_mode: bool,

    cycle: u16,
}

impl APU2A03 {
    pub fn new() -> Self {
        Self {
            square_pulse_1: Arc::new(Mutex::new(LengthCountedChannel::new(SquarePulse::new(
                440., 0.5, 20,
            )))),
            square_pulse_1_timer: 0,
            square_pulse_2: Arc::new(Mutex::new(LengthCountedChannel::new(SquarePulse::new(
                440., 0.5, 20,
            )))),
            square_pulse_2_timer: 0,

            reference_clock_frequency: 1.789773 * 1E6,

            is_4_step_squence_mode: false,

            cycle: 0,
        }
    }

    pub fn get_square_pulse_1_player(&self) -> APUChannelPlayer<LengthCountedChannel<SquarePulse>> {
        APUChannelPlayer::from_clone(self.square_pulse_1.clone())
    }

    pub fn get_square_pulse_2_player(&self) -> APUChannelPlayer<LengthCountedChannel<SquarePulse>> {
        APUChannelPlayer::from_clone(self.square_pulse_2.clone())
    }

    pub fn update_reference_frequency(&mut self, freq: f32) {
        self.reference_clock_frequency = freq;
    }

    pub(crate) fn read_register(&self, register: Register) -> u8 {
        match register {
            Register::Status => {
                let sqr1_length_counter = if let Ok(square_pulse_1) = self.square_pulse_1.lock() {
                    (square_pulse_1.length_counter().counter() != 0) as u8
                } else {
                    0
                };
                let sqr2_length_counter = if let Ok(square_pulse_2) = self.square_pulse_2.lock() {
                    (square_pulse_2.length_counter().counter() != 0) as u8
                } else {
                    0
                };

                sqr2_length_counter << 1 | sqr1_length_counter
            }
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
                    square_pulse_1.channel_mut().set_duty_cycle(duty_cycle);
                    square_pulse_1.channel_mut().set_volume(volume, use_volume);
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

                if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
                    square_pulse_1
                        .length_counter_mut()
                        .reload_counter(data >> 3);
                }
            }
            Register::Pulse2_1 => {
                let duty_cycle = [0.125, 0.25, 0.5, 0.75][data as usize >> 6];
                let volume = data & 0xF;
                let use_volume = data & 0x10 != 0;

                if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
                    square_pulse_2.channel_mut().set_duty_cycle(duty_cycle);
                    square_pulse_2.channel_mut().set_volume(volume, use_volume);
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

                if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
                    square_pulse_2
                        .length_counter_mut()
                        .reload_counter(data >> 3);
                }
            }
            Register::Triangle1 => {}
            Register::Triangle2 => {}
            Register::Triangle3 => {}
            Register::Triangle4 => {}
            Register::Noise1 => {}
            Register::Noise2 => {}
            Register::Noise3 => {}
            Register::Noise4 => {}
            Register::DMC1 => {}
            Register::DMC2 => {}
            Register::DMC3 => {}
            Register::DMC4 => {}
            Register::Status => {
                // enable and disable length counters
                if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
                    square_pulse_1
                        .length_counter_mut()
                        .set_enabled((data >> 0 & 1) != 0);
                }
                if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
                    square_pulse_2
                        .length_counter_mut()
                        .set_enabled((data >> 1 & 1) != 0);
                }
            }
            Register::FrameCounter => {
                self.is_4_step_squence_mode = data & 0x80 == 0;
            }
        }
    }

    fn update_sqaure_pulse_1_frequency(&mut self) {
        if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
            square_pulse_1.channel_mut().set_freq(
                self.reference_clock_frequency / (16 * (self.square_pulse_1_timer + 1)) as f32,
            );
        }
    }

    fn update_sqaure_pulse_2_frequency(&mut self) {
        if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
            square_pulse_2.channel_mut().set_freq(
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

    fn square_pulse_1_length_counter_decrement(&mut self) {
        if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
            square_pulse_1.length_counter_mut().decrement();
        }
    }

    fn square_pulse_2_length_counter_decrement(&mut self) {
        if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
            square_pulse_2.length_counter_mut().decrement();
        }
    }

    pub fn clock(&mut self) {
        self.cycle += 1;

        match self.cycle {
            3729 => {}
            7457 => {
                self.square_pulse_1_length_counter_decrement();
                self.square_pulse_2_length_counter_decrement();
            }
            11186 => {}
            14915 if self.is_4_step_squence_mode => {
                self.square_pulse_1_length_counter_decrement();
                self.square_pulse_2_length_counter_decrement();
                self.cycle = 0;
            }
            18641 if !self.is_4_step_squence_mode => {
                self.square_pulse_1_length_counter_decrement();
                self.square_pulse_2_length_counter_decrement();
                self.cycle = 0;
            }
            _ => {
                // ignore
            }
        }
    }
}
