use crate::apu2a03_registers::Register;
use crate::channels::SquarePulse;
use crate::length_counter::LengthCountedChannel;
use crate::sweeper::Sweeper;
use crate::tone_source::APUChannelPlayer;
use std::sync::{Arc, Mutex};

pub struct APU2A03 {
    square_pulse_1: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_1_sweeper: Sweeper,
    square_pulse_2: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_2_sweeper: Sweeper,

    reference_clock_frequency: f32,

    is_4_step_squence_mode: bool,

    cycle: u16,
}

impl APU2A03 {
    pub fn new() -> Self {
        let square_pulse_1 = Arc::new(Mutex::new(LengthCountedChannel::new(SquarePulse::new(
            440.,
            0.5,
            20,
            1.789773 * 1E6,
        ))));
        let square_pulse_2 = Arc::new(Mutex::new(LengthCountedChannel::new(SquarePulse::new(
            440.,
            0.5,
            20,
            1.789773 * 1E6,
        ))));
        Self {
            square_pulse_1: square_pulse_1.clone(),
            square_pulse_1_sweeper: Sweeper::new(square_pulse_1.clone()),
            square_pulse_2: square_pulse_2.clone(),
            square_pulse_2_sweeper: Sweeper::new(square_pulse_2.clone()),

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
                self.square_pulse_1_sweeper.set_from_data_byte(data);
            }
            Register::Pulse1_3 => {
                if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
                    let period = square_pulse_1.channel().get_period();

                    // lower timer bits
                    square_pulse_1
                        .channel_mut()
                        .set_period((period & 0xFF00) | data as u16);
                }
            }
            Register::Pulse1_4 => {
                if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
                    square_pulse_1
                        .length_counter_mut()
                        .reload_counter(data >> 3);

                    let period = square_pulse_1.channel().get_period();

                    // high timer bits
                    square_pulse_1
                        .channel_mut()
                        .set_period((period & 0xFF) | ((data as u16 & 0b111) << 8))
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
                self.square_pulse_2_sweeper.set_from_data_byte(data);
            }
            Register::Pulse2_3 => {
                if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
                    let period = square_pulse_2.channel().get_period();

                    // lower timer bits
                    square_pulse_2
                        .channel_mut()
                        .set_period((period & 0xFF00) | data as u16);
                }
            }
            Register::Pulse2_4 => {
                if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
                    square_pulse_2
                        .length_counter_mut()
                        .reload_counter(data >> 3);

                    let period = square_pulse_2.channel().get_period();

                    // high timer bits
                    square_pulse_2
                        .channel_mut()
                        .set_period((period & 0xFF) | ((data as u16 & 0b111) << 8));
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
                self.square_pulse_1_sweeper.clock();
                self.square_pulse_2_length_counter_decrement();
                self.square_pulse_2_sweeper.clock();
            }
            11186 => {}
            14915 if self.is_4_step_squence_mode => {
                self.square_pulse_1_length_counter_decrement();
                self.square_pulse_1_sweeper.clock();
                self.square_pulse_2_length_counter_decrement();
                self.square_pulse_2_sweeper.clock();
                self.cycle = 0;
            }
            18641 if !self.is_4_step_squence_mode => {
                self.square_pulse_1_length_counter_decrement();
                self.square_pulse_1_sweeper.clock();
                self.square_pulse_2_length_counter_decrement();
                self.square_pulse_2_sweeper.clock();
                self.cycle = 0;
            }
            _ => {
                // ignore
            }
        }
    }
}
