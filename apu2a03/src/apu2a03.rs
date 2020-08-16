use crate::apu2a03_registers::Register;
use crate::channels::{Dmc, NoiseWave, SquarePulse, TriangleWave};
use crate::envelope::EnvelopedChannel;
use crate::length_counter::LengthCountedChannel;
use crate::mixer::Mixer;
use crate::sweeper::Sweeper;
use crate::tone_source::{APUChannel, APUChannelPlayer, BufferedChannel, Filter, TimedAPUChannel};
use common::interconnection::{APUCPUConnection, CpuIrqProvider};
use std::cell::Cell;
use std::sync::{Arc, Mutex};

pub struct APU2A03 {
    square_pulse_1: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_1_sweeper: Sweeper,
    square_pulse_2: Arc<Mutex<LengthCountedChannel<SquarePulse>>>,
    square_pulse_2_sweeper: Sweeper,

    triangle: Arc<Mutex<LengthCountedChannel<TriangleWave>>>,

    noise: Arc<Mutex<LengthCountedChannel<NoiseWave>>>,

    dmc: Arc<Mutex<Dmc>>,

    buffered_channel: Arc<Mutex<BufferedChannel>>,

    mixer: Mixer,

    is_4_step_squence_mode: bool,
    interrupt_inhibit_flag: bool,

    cycle: u16,

    wait_reset: i8,

    sample_counter: f64,

    interrupt_flag: Cell<bool>,
    request_interrupt_flag_change: Cell<bool>,

    filter: Filter,
    filter_counter: u8,
}

impl APU2A03 {
    pub fn new() -> Self {
        let square_pulse_1 = Arc::new(Mutex::new(LengthCountedChannel::new(SquarePulse::new())));
        let square_pulse_2 = Arc::new(Mutex::new(LengthCountedChannel::new(SquarePulse::new())));
        let triangle = Arc::new(Mutex::new(LengthCountedChannel::new(TriangleWave::new())));
        let noise = Arc::new(Mutex::new(LengthCountedChannel::new(NoiseWave::new())));
        let dmc = Arc::new(Mutex::new(Dmc::new()));

        Self {
            square_pulse_1: square_pulse_1.clone(),
            square_pulse_1_sweeper: Sweeper::new(square_pulse_1.clone()),
            square_pulse_2: square_pulse_2.clone(),
            square_pulse_2_sweeper: Sweeper::new(square_pulse_2.clone()),

            triangle: triangle.clone(),

            noise: noise.clone(),

            dmc: dmc.clone(),

            buffered_channel: Arc::new(Mutex::new(BufferedChannel::new())),

            mixer: Mixer::new(
                square_pulse_1.clone(),
                square_pulse_2.clone(),
                triangle.clone(),
                noise.clone(),
                dmc.clone(),
            ),

            is_4_step_squence_mode: false,
            interrupt_inhibit_flag: false,

            cycle: 0,

            sample_counter: 0.,

            wait_reset: 0,

            interrupt_flag: Cell::new(false),
            request_interrupt_flag_change: Cell::new(false),

            filter: Filter::new(),
            filter_counter: 0,
        }
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
                let triangle_length_counter = if let Ok(triangle) = self.triangle.lock() {
                    (triangle.length_counter().counter() != 0) as u8
                } else {
                    0
                };
                let noise_length_counter = if let Ok(noise) = self.noise.lock() {
                    (noise.length_counter().counter() != 0) as u8
                } else {
                    0
                };
                let mut dmc_active = 0;
                let mut dmc_interrupt = 0;
                if let Ok(dmc) = self.dmc.lock() {
                    dmc_active = dmc.sample_remaining_bytes_more_than_0() as u8;
                    dmc_interrupt = dmc.get_irq_pin_state() as u8;
                }

                let frame_interrupt = self.interrupt_flag.get() as u8;
                self.interrupt_flag.set(false);
                self.request_interrupt_flag_change.set(true);

                dmc_interrupt << 7
                    | frame_interrupt << 6
                    | dmc_active << 4
                    | noise_length_counter << 3
                    | triangle_length_counter << 2
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
                if let Ok(mut triangle) = self.triangle.lock() {
                    triangle
                        .channel_mut()
                        .set_linear_counter_reload_value(data & 0x7F);
                    triangle
                        .channel_mut()
                        .set_linear_counter_control_flag(data & 0x80 != 0);

                    triangle.length_counter_mut().set_halt(data & 0x80 != 0);
                }
            }
            Register::Triangle2 => {
                // unused
            }
            Register::Triangle3 => {
                if let Ok(mut triangle) = self.triangle.lock() {
                    let period = triangle.channel().get_period();

                    // lower timer bits
                    triangle
                        .channel_mut()
                        .set_period((period & 0xFF00) | data as u16);
                }
            }
            Register::Triangle4 => {
                if let Ok(mut triangle) = self.triangle.lock() {
                    triangle.length_counter_mut().reload_counter(data >> 3);

                    let period = triangle.channel().get_period();

                    // high timer bits
                    triangle
                        .channel_mut()
                        .set_period((period & 0xFF) | ((data as u16 & 0b111) << 8));

                    triangle.channel_mut().set_linear_counter_reload_flag(true);
                }
            }
            Register::Noise1 => {
                let volume = data & 0xF;
                let use_volume = data & 0x10 != 0;
                let halt = data & 0x20 != 0;

                if let Ok(mut noise) = self.noise.lock() {
                    noise
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_volume(volume, use_volume);
                    noise.length_counter_mut().set_halt(halt);
                    noise
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_loop_flag(halt);
                    noise
                        .channel_mut()
                        .envelope_generator_mut()
                        .set_start_flag(true);
                }
            }
            Register::Noise2 => {
                // unused
            }
            Register::Noise3 => {
                if let Ok(mut noise) = self.noise.lock() {
                    let channel = noise.channel_mut();

                    channel.set_mode_flag(data & 0x80 != 0);
                    channel.set_period(data & 0xF);
                }
            }
            Register::Noise4 => {
                if let Ok(mut noise) = self.noise.lock() {
                    noise.length_counter_mut().reload_counter(data >> 3);
                }
            }
            Register::DMC1 => {
                let rate_index = data & 0xF;
                let loop_flag = data & 0x40 != 0;
                let irq_enabled = data & 0x80 != 0;

                if let Ok(mut dmc) = self.dmc.lock() {
                    dmc.set_rate_index(rate_index);
                    dmc.set_loop_flag(loop_flag);
                    dmc.set_irq_enabled_flag(irq_enabled);
                }
            }
            Register::DMC2 => {
                if let Ok(mut dmc) = self.dmc.lock() {
                    dmc.set_direct_output_level_load(data & 0x7F);
                }
            }
            Register::DMC3 => {
                if let Ok(mut dmc) = self.dmc.lock() {
                    dmc.set_samples_address(data);
                }
            }
            Register::DMC4 => {
                if let Ok(mut dmc) = self.dmc.lock() {
                    dmc.set_samples_length(data);
                }
            }
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
                if let Ok(mut triangle) = self.triangle.lock() {
                    triangle
                        .length_counter_mut()
                        .set_enabled((data >> 2 & 1) != 0);
                }
                if let Ok(mut noise) = self.noise.lock() {
                    noise.length_counter_mut().set_enabled((data >> 3 & 1) != 0);
                }
                if let Ok(mut dmc) = self.dmc.lock() {
                    if data >> 4 & 1 == 0 {
                        dmc.clear_sample_remaining_bytes_and_silence();
                    } else {
                        if !dmc.sample_remaining_bytes_more_than_0() {
                            dmc.restart_sample();
                        }
                    }

                    dmc.clear_interrupt_flag();
                }
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
        sink.set_volume(0.15);

        sink.play();
        sink.detach();
    }

    fn length_counter_decrement<S: APUChannel>(channel: &mut Arc<Mutex<LengthCountedChannel<S>>>) {
        if let Ok(mut channel) = channel.lock() {
            channel.length_counter_mut().decrement();
        }
    }

    fn envelope_clock<S: EnvelopedChannel>(channel: &mut Arc<Mutex<S>>) {
        if let Ok(mut channel) = channel.lock() {
            channel.clock_envlope();
        }
    }

    fn timer_clock<S: TimedAPUChannel>(channel: &mut Arc<Mutex<S>>) {
        if let Ok(mut channel) = channel.lock() {
            channel.timer_clock();
        }
    }

    fn triangle_linear_counter_clock(&mut self) {
        if let Ok(mut triangle) = self.triangle.lock() {
            triangle.channel_mut().clock_linear_counter();
        }
    }

    fn generate_quarter_frame_clock(&mut self) {
        Self::envelope_clock(&mut self.square_pulse_1);
        Self::envelope_clock(&mut self.square_pulse_2);
        Self::envelope_clock(&mut self.noise);
        self.triangle_linear_counter_clock();
    }

    fn generate_half_frame_clock(&mut self) {
        Self::length_counter_decrement(&mut self.square_pulse_1);
        self.square_pulse_1_sweeper.clock();
        Self::length_counter_decrement(&mut self.square_pulse_2);
        self.square_pulse_2_sweeper.clock();
        Self::length_counter_decrement(&mut self.triangle);
        Self::length_counter_decrement(&mut self.noise);
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

        let cpu = 1.789773 * 1E6;
        let apu = cpu / 2.;

        // after how many apu clocks a sample should be recorded
        // for now its 44100 * 8 and that is only due to the filter used, as it supports
        // that only for now.
        //
        // FIXME: the buffer is being emptied faster than filled for some reason, please investigate
        //  (-0.9) is set to fix that, but of course its not 1% reliable :(
        let samples_every_n_apu_clock = (apu / (crate::SAMPLE_RATE as f64 * 8.)) - 0.8;

        self.sample_counter += 1.0;
        if self.sample_counter >= samples_every_n_apu_clock {
            let output = self.mixer.get_output();
            let output = self.filter.apply(output);

            if self.filter_counter == 8 {
                self.filter_counter = 0;
                self.buffered_channel.lock().unwrap().recored_sample(output);
            } else {
                self.filter_counter += 1;
            }

            self.sample_counter -= samples_every_n_apu_clock;
        }

        Self::timer_clock(&mut self.square_pulse_1);
        Self::timer_clock(&mut self.square_pulse_2);
        Self::timer_clock(&mut self.triangle);
        Self::timer_clock(&mut self.triangle);
        Self::timer_clock(&mut self.noise);
        Self::timer_clock(&mut self.dmc);

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
        let dmc_irq_request = if let Ok(dmc) = self.dmc.lock() {
            dmc.is_irq_change_requested()
        } else {
            false
        };

        self.request_interrupt_flag_change.get() || dmc_irq_request
    }

    fn irq_pin_state(&self) -> bool {
        let dmc_irq = if let Ok(dmc) = self.dmc.lock() {
            dmc.get_irq_pin_state()
        } else {
            false
        };

        self.interrupt_flag.get() || dmc_irq
    }

    fn clear_irq_request_pin(&mut self) {
        self.request_interrupt_flag_change.set(false);

        if let Ok(mut dmc) = self.dmc.lock() {
            dmc.clear_irq_request_pin();
        }
    }
}

impl APUCPUConnection for APU2A03 {
    fn request_dmc_reader_read(&self) -> Option<u16> {
        if let Ok(dmc) = self.dmc.lock() {
            dmc.request_dmc_reader_read()
        } else {
            None
        }
    }

    fn submit_buffer_byte(&mut self, byte: u8) {
        if let Ok(mut dmc) = self.dmc.lock() {
            dmc.submit_buffer_byte(byte);
        }
    }
}
