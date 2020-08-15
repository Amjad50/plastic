use crate::apu2a03_registers::Register;
use crate::channels::SquarePulse;
use crate::envelope::EnvelopedChannel;
use crate::length_counter::LengthCountedChannel;
use crate::sweeper::Sweeper;
use crate::tone_source::{APUChannel, APUChannelPlayer, BufferedChannel};
use common::interconnection::CpuIrqProvider;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

pub struct APU2A03 {
    square_pulse_1: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_1_sweeper: Sweeper,
    square_pulse_2: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_2_sweeper: Sweeper,

    buffered_channel: Arc<Mutex<BufferedChannel>>,

    // triangle: Arc<Mutex<LengthCountedChannel<TriangleWave>>>,

    // noise: Arc<Mutex<LengthCountedChannel<NoiseWave>>>,
    mixer: rodio::dynamic_mixer::DynamicMixer<f32>,

    reference_clock_frequency: f32,

    is_4_step_squence_mode: bool,
    interrupt_inhibit_flag: bool,

    cycle: u16,

    wait_reset: i8,

    sample_counter: f32,

    interrupt_flag: Cell<bool>,
    request_interrupt_flag_change: Cell<bool>,
}

impl APU2A03 {
    pub fn new() -> Self {
        let square_pulse_1 = Arc::new(Mutex::new(LengthCountedChannel::new(SquarePulse::new())));
        let square_pulse_2 = Arc::new(Mutex::new(LengthCountedChannel::new(SquarePulse::new())));

        let buffered_channel = Arc::new(Mutex::new(BufferedChannel::new()));

        let sqr1 = APUChannelPlayer::from_clone(square_pulse_1.clone());
        let sqr2 = APUChannelPlayer::from_clone(square_pulse_2.clone());
        // let triangle = APUChannelPlayer::from_clone(self.triangle.clone());
        // let noise = APUChannelPlayer::from_clone(self.noise.clone());

        let (controller, mixer) = rodio::dynamic_mixer::mixer::<f32>(5, crate::SAMPLE_RATE);

        controller.add(sqr1);
        controller.add(sqr2);
        // controller.add(triangle);
        // controller.add(noise);

        Self {
            square_pulse_1: square_pulse_1.clone(),
            square_pulse_1_sweeper: Sweeper::new(square_pulse_1.clone()),
            square_pulse_2: square_pulse_2.clone(),
            square_pulse_2_sweeper: Sweeper::new(square_pulse_2.clone()),

            buffered_channel,

            // triangle: Arc::new(Mutex::new(LengthCountedChannel::new(TriangleWave::new(
            //     440.,
            //     20,
            //     1.789773 * 1E6,
            // )))),

            // noise: Arc::new(Mutex::new(LengthCountedChannel::new(
            //     EnvelopeGenerator::new(NoiseWave::new(1.789773 * 1E6)),
            // ))),
            mixer,
            reference_clock_frequency: 1.789773 * 1E6,

            is_4_step_squence_mode: false,
            interrupt_inhibit_flag: false,

            cycle: 0,

            sample_counter: 0.,

            wait_reset: 0,

            interrupt_flag: Cell::new(false),
            request_interrupt_flag_change: Cell::new(false),
        }
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
                // let triangle_length_counter = if let Ok(triangle) = self.triangle.lock() {
                //     (triangle.length_counter().counter() != 0) as u8
                // } else {
                //     0
                // };
                // let noise_length_counter = if let Ok(noise) = self.noise.lock() {
                //     (noise.length_counter().counter() != 0) as u8
                // } else {
                //     0
                // };

                let frame_interrupt = self.interrupt_flag.get() as u8;
                self.interrupt_flag.set(false);
                self.request_interrupt_flag_change.set(true);

                frame_interrupt << 6
                //     | noise_length_counter << 3
                //     | triangle_length_counter << 2
                    | sqr2_length_counter << 1
                    | sqr1_length_counter
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
                let duty_cycle_index = data >> 6;
                let volume = data & 0xF;
                let use_volume = data & 0x10 != 0;
                let halt = data & 0x20 != 0;

                if let Ok(mut square_pulse_1) = self.square_pulse_1.lock() {
                    square_pulse_1
                        .channel_mut()
                        .set_duty_cycle_index(duty_cycle_index);
                    square_pulse_1
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_volume(volume, use_volume);

                    square_pulse_1.length_counter_mut().set_halt(halt);
                    square_pulse_1
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_loop_flag(halt);

                    square_pulse_1
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_start_flag(true);
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
                        .set_period((period & 0xFF) | ((data as u16 & 0b111) << 8));

                    square_pulse_1
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_start_flag(true);

                    // reset pulse
                    square_pulse_1.channel_mut().reset();
                }
            }
            Register::Pulse2_1 => {
                let duty_cycle_index = data >> 6;
                let volume = data & 0xF;
                let use_volume = data & 0x10 != 0;
                let halt = data & 0x20 != 0;

                if let Ok(mut square_pulse_2) = self.square_pulse_2.lock() {
                    square_pulse_2
                        .channel_mut()
                        .set_duty_cycle_index(duty_cycle_index);
                    square_pulse_2
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_volume(volume, use_volume);

                    square_pulse_2.length_counter_mut().set_halt(halt);
                    square_pulse_2
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_loop_flag(halt);
                    square_pulse_2
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_start_flag(true);
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

                    square_pulse_2
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_start_flag(true);

                    // reset pulse
                    square_pulse_2.channel_mut().reset();
                }
            }
            Register::Triangle1 => {
                // if let Ok(mut triangle) = self.triangle.lock() {
                //     triangle
                //         .channel_mut()
                //         .set_linear_counter_reload_value(data & 0x7F);
                //     triangle
                //         .channel_mut()
                //         .set_linear_counter_control_flag(data & 0x80 != 0);

                //     triangle.length_counter_mut().set_halt(data & 0x80 != 0);
                // }
            }
            Register::Triangle2 => {
                // unused
            }
            Register::Triangle3 => {
                // if let Ok(mut triangle) = self.triangle.lock() {
                //     let period = triangle.channel().get_period();

                //     // lower timer bits
                //     triangle
                //         .channel_mut()
                //         .set_period((period & 0xFF00) | data as u16);
                // }
            }
            Register::Triangle4 => {
                // if let Ok(mut triangle) = self.triangle.lock() {
                //     triangle.length_counter_mut().reload_counter(data >> 3);

                //     let period = triangle.channel().get_period();

                //     // high timer bits
                //     triangle
                //         .channel_mut()
                //         .set_period((period & 0xFF) | ((data as u16 & 0b111) << 8));

                //     triangle.channel_mut().set_linear_counter_reload_flag(true);
                // }
            }
            Register::Noise1 => {
                // let volume = data & 0xF;
                // let use_volume = data & 0x10 != 0;
                // let halt = data & 0x20 != 0;

                // if let Ok(mut noise) = self.noise.lock() {
                //     noise.channel_mut().set_volume(volume, use_volume);
                //     noise.length_counter_mut().set_halt(halt);
                //     noise.channel_mut().set_loop_flag(halt);
                //     noise.channel_mut().set_start_flag(true);
                // }
            }
            Register::Noise2 => {
                // unused
            }
            Register::Noise3 => {
                //if let Ok(mut noise) = self.noise.lock() {
                //    let channel = noise.channel_mut().channel_mut();

                //    channel.set_mode_flag(data & 0x80 != 0);
                //    channel.set_period(data & 0xF);
                //}
            }
            Register::Noise4 => {
                // if let Ok(mut noise) = self.noise.lock() {
                //     noise.length_counter_mut().reload_counter(data >> 3);
                //     noise.channel_mut().channel_mut().reset();
                // }
            }
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
                // if let Ok(mut triangle) = self.triangle.lock() {
                //     triangle
                //         .length_counter_mut()
                //         .set_enabled((data >> 2 & 1) != 0);
                // }
                // if let Ok(mut noise) = self.noise.lock() {
                //     noise.length_counter_mut().set_enabled((data >> 3 & 1) != 0);
                // }
            }
            Register::FrameCounter => {
                self.is_4_step_squence_mode = data & 0x80 == 0;
                self.interrupt_inhibit_flag = data & 0x40 != 0;

                if self.interrupt_inhibit_flag {
                    self.interrupt_flag.set(false);
                    self.request_interrupt_flag_change.set(true);
                }

                // clock immediately
                if data & 0x80 != 0 {
                    self.generate_half_frame_clock();
                    self.generate_quarter_frame_clock();
                } else {
                    // reset(side effect)
                    self.wait_reset = 2; // after 4 CPU clocks
                }
            }
        }
    }

    pub fn play(&self) {
        let device = rodio::default_output_device().unwrap();
        let sink = rodio::Sink::new(&device);

        sink.append(APUChannelPlayer::from_clone(self.buffered_channel.clone()));
        sink.set_volume(0.05);

        sink.play();
        sink.detach();
    }

    fn length_counter_decrement<S: APUChannel>(channel: &mut Arc<Mutex<LengthCountedChannel<S>>>) {
        if let Ok(mut channel) = channel.lock() {
            channel.length_counter_mut().decrement();
        }
    }

    fn envelope_clock<S: EnvelopedChannel>(channel: &mut Arc<Mutex<LengthCountedChannel<S>>>) {
        if let Ok(mut channel) = channel.lock() {
            channel.channel_mut().clock_envlope();
        }
    }

    fn timer_clock<S: APUChannel>(channel: &mut Arc<Mutex<LengthCountedChannel<S>>>) {
        if let Ok(mut channel) = channel.lock() {
            channel.channel_mut().timer_clock();
        }
    }

    fn triangle_linear_counter_clock(&mut self) {
        // if let Ok(mut triangle) = self.triangle.lock() {
        //     triangle.channel_mut().clock_linear_counter();
        // }
    }

    fn generate_quarter_frame_clock(&mut self) {
        Self::envelope_clock(&mut self.square_pulse_1);
        Self::envelope_clock(&mut self.square_pulse_2);
        // Self::envelope_clock(&mut self.noise);
        self.triangle_linear_counter_clock();
    }

    fn generate_half_frame_clock(&mut self) {
        Self::length_counter_decrement(&mut self.square_pulse_1);
        self.square_pulse_1_sweeper.clock();
        Self::length_counter_decrement(&mut self.square_pulse_2);
        self.square_pulse_2_sweeper.clock();
        // Self::length_counter_decrement(&mut self.triangle);
        // Self::length_counter_decrement(&mut self.noise);
    }

    pub fn clock(&mut self) {
        if self.wait_reset > 0 {
            self.wait_reset -= 1;
        } else if self.wait_reset == 0 {
            self.cycle = 0;
            self.wait_reset = -1;

            // mode bit is set
            if !self.is_4_step_squence_mode {
                self.generate_quarter_frame_clock();
                self.generate_half_frame_clock();
            }
        }

        let samples_per_frame = 894886.5 / crate::SAMPLE_RATE as f32;

        self.sample_counter += 1.0;
        if self.sample_counter >= samples_per_frame {
            self.buffered_channel
                .lock()
                .unwrap()
                .recored_sample(self.mixer.next().unwrap());
            self.sample_counter -= samples_per_frame;
        }

        // if let Ok(mut noise) = self.noise.lock() {
        //     noise.channel_mut().clock_timer();
        // }
        Self::timer_clock(&mut self.square_pulse_1);
        Self::timer_clock(&mut self.square_pulse_2);

        self.cycle += 1;

        match self.cycle {
            3729 => {
                self.generate_quarter_frame_clock();
            }
            7457 => {
                self.generate_quarter_frame_clock();
                self.generate_half_frame_clock();
            }
            11186 => {
                self.generate_quarter_frame_clock();
            }
            14915 if self.is_4_step_squence_mode => {
                self.generate_quarter_frame_clock();
                self.generate_half_frame_clock();

                if !self.interrupt_inhibit_flag {
                    self.interrupt_flag.set(true);
                    self.request_interrupt_flag_change.set(true);
                }
                self.cycle = 0;
            }
            18641 if !self.is_4_step_squence_mode => {
                self.generate_quarter_frame_clock();
                self.generate_half_frame_clock();
                self.cycle = 0;
            }
            _ => {
                // ignore
            }
        }
    }
}

impl CpuIrqProvider for APU2A03 {
    fn is_irq_change_requested(&self) -> bool {
        self.request_interrupt_flag_change.get()
    }

    fn irq_pin_state(&self) -> bool {
        self.interrupt_flag.get()
    }

    fn clear_irq_request_pin(&mut self) {
        self.request_interrupt_flag_change.set(false);
    }
}
