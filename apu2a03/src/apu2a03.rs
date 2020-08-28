use crate::apu2a03_registers::Register;
use crate::channels::{Dmc, NoiseWave, SquarePulse, TriangleWave};
use crate::envelope::EnvelopedChannel;
use crate::length_counter::LengthCountedChannel;
use crate::mixer::Mixer;
use crate::tone_source::{APUChannel, APUChannelPlayer, BufferedChannel, TimedAPUChannel};
use common::interconnection::{APUCPUConnection, CpuIrqProvider};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use rodio::DeviceTrait;

pub struct APU2A03 {
    square_pulse_1: Rc<RefCell<LengthCountedChannel<SquarePulse>>>,
    square_pulse_2: Rc<RefCell<LengthCountedChannel<SquarePulse>>>,

    triangle: Rc<RefCell<LengthCountedChannel<TriangleWave>>>,

    noise: Rc<RefCell<LengthCountedChannel<NoiseWave>>>,

    dmc: Rc<RefCell<Dmc>>,

    buffered_channel: Arc<Mutex<BufferedChannel>>,

    mixer: Mixer,

    is_4_step_squence_mode: bool,
    interrupt_inhibit_flag: bool,

    cycle: u16,

    wait_reset: i8,

    apu_freq: f64,
    sample_counter: f64,

    offset: f64,

    interrupt_flag: Cell<bool>,
    request_interrupt_flag_change: Cell<bool>,

    player: Option<rodio::Sink>,
}

impl APU2A03 {
    pub fn new() -> Self {
        let square_pulse_1 = Rc::new(RefCell::new(LengthCountedChannel::new(SquarePulse::new(
            true,
        ))));
        let square_pulse_2 = Rc::new(RefCell::new(LengthCountedChannel::new(SquarePulse::new(
            false,
        ))));
        let triangle = Rc::new(RefCell::new(LengthCountedChannel::new(TriangleWave::new())));
        let noise = Rc::new(RefCell::new(LengthCountedChannel::new(NoiseWave::new())));
        let dmc = Rc::new(RefCell::new(Dmc::new()));

        let buffered_channel = Arc::new(Mutex::new(BufferedChannel::new()));

        Self {
            square_pulse_1: square_pulse_1.clone(),
            square_pulse_2: square_pulse_2.clone(),

            triangle: triangle.clone(),

            noise: noise.clone(),

            dmc: dmc.clone(),

            buffered_channel: buffered_channel.clone(),

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

            apu_freq: 0.,
            sample_counter: 0.,

            offset: 0.5,

            wait_reset: 0,

            interrupt_flag: Cell::new(false),
            request_interrupt_flag_change: Cell::new(false),

            player: Self::get_player(buffered_channel.clone()),
        }
    }

    fn get_player<S: APUChannel + Send + 'static>(channel: Arc<Mutex<S>>) -> Option<rodio::Sink> {
        let device = rodio::default_output_device()?;

        // bug in rodio, that it panics if the device does not support any format
        // it is fixed now in github, not sure when is the release coming
        let formats = device.supported_output_formats().ok()?;
        if formats.count() > 0 {
            let sink = rodio::Sink::new(&device);

            let low_pass_player = rodio::source::Source::low_pass(
                APUChannelPlayer::from_clone(channel.clone()),
                10000,
            );

            sink.append(low_pass_player);
            sink.set_volume(0.15);

            sink.pause();

            Some(sink)
        } else {
            None
        }
    }

    pub(crate) fn read_register(&self, register: Register) -> u8 {
        match register {
            Register::Status => {
                let sqr1_length_counter =
                    (self.square_pulse_1.borrow().length_counter().counter() != 0) as u8;

                let sqr2_length_counter =
                    (self.square_pulse_2.borrow().length_counter().counter() != 0) as u8;

                let triangle_length_counter =
                    (self.triangle.borrow().length_counter().counter() != 0) as u8;

                let noise_length_counter =
                    (self.noise.borrow().length_counter().counter() != 0) as u8;

                let dmc = self.dmc.borrow();
                let dmc_active = dmc.sample_remaining_bytes_more_than_0() as u8;
                let dmc_interrupt = dmc.get_irq_pin_state() as u8;

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

                let mut square_pulse_1 = self.square_pulse_1.borrow_mut();

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
            Register::Pulse1_2 => {
                // sweep
                self.square_pulse_1
                    .borrow_mut()
                    .channel_mut()
                    .set_sweeper_data(data);
            }
            Register::Pulse1_3 => {
                let mut square_pulse_1 = self.square_pulse_1.borrow_mut();

                let period = square_pulse_1.channel().get_period();

                // lower timer bits
                square_pulse_1
                    .channel_mut()
                    .set_period((period & 0xFF00) | data as u16);
            }
            Register::Pulse1_4 => {
                let mut square_pulse_1 = self.square_pulse_1.borrow_mut();

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
            Register::Pulse2_1 => {
                let duty_cycle_index = data >> 6;
                let volume = data & 0xF;
                let use_volume = data & 0x10 != 0;
                let halt = data & 0x20 != 0;

                let mut square_pulse_2 = self.square_pulse_2.borrow_mut();

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
            Register::Pulse2_2 => {
                // sweep
                self.square_pulse_2
                    .borrow_mut()
                    .channel_mut()
                    .set_sweeper_data(data);
            }
            Register::Pulse2_3 => {
                let mut square_pulse_2 = self.square_pulse_2.borrow_mut();

                let period = square_pulse_2.channel().get_period();

                // lower timer bits
                square_pulse_2
                    .channel_mut()
                    .set_period((period & 0xFF00) | data as u16);
            }
            Register::Pulse2_4 => {
                let mut square_pulse_2 = self.square_pulse_2.borrow_mut();

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
            Register::Triangle1 => {
                let mut triangle = self.triangle.borrow_mut();
                triangle
                    .channel_mut()
                    .set_linear_counter_reload_value(data & 0x7F);
                triangle
                    .channel_mut()
                    .set_linear_counter_control_flag(data & 0x80 != 0);

                triangle.length_counter_mut().set_halt(data & 0x80 != 0);
            }
            Register::Triangle2 => {
                // unused
            }
            Register::Triangle3 => {
                let mut triangle = self.triangle.borrow_mut();

                let period = triangle.channel().get_period();

                // lower timer bits
                triangle
                    .channel_mut()
                    .set_period((period & 0xFF00) | data as u16);
            }
            Register::Triangle4 => {
                let mut triangle = self.triangle.borrow_mut();

                triangle.length_counter_mut().reload_counter(data >> 3);

                let period = triangle.channel().get_period();

                // high timer bits
                triangle
                    .channel_mut()
                    .set_period((period & 0xFF) | ((data as u16 & 0b111) << 8));

                triangle.channel_mut().set_linear_counter_reload_flag(true);
            }
            Register::Noise1 => {
                let volume = data & 0xF;
                let use_volume = data & 0x10 != 0;
                let halt = data & 0x20 != 0;

                let mut noise = self.noise.borrow_mut();
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
            Register::Noise2 => {
                // unused
            }
            Register::Noise3 => {
                let mut noise = self.noise.borrow_mut();

                let channel = noise.channel_mut();

                channel.set_mode_flag(data & 0x80 != 0);
                channel.set_period(data & 0xF);
            }
            Register::Noise4 => {
                self.noise
                    .borrow_mut()
                    .length_counter_mut()
                    .reload_counter(data >> 3);
            }
            Register::DMC1 => {
                let rate_index = data & 0xF;
                let loop_flag = data & 0x40 != 0;
                let irq_enabled = data & 0x80 != 0;

                let mut dmc = self.dmc.borrow_mut();
                dmc.set_rate_index(rate_index);
                dmc.set_loop_flag(loop_flag);
                dmc.set_irq_enabled_flag(irq_enabled);
            }
            Register::DMC2 => {
                self.dmc
                    .borrow_mut()
                    .set_direct_output_level_load(data & 0x7F);
            }
            Register::DMC3 => {
                self.dmc.borrow_mut().set_samples_address(data);
            }
            Register::DMC4 => {
                self.dmc.borrow_mut().set_samples_length(data);
            }
            Register::Status => {
                // enable and disable length counters
                self.square_pulse_1
                    .borrow_mut()
                    .length_counter_mut()
                    .set_enabled((data >> 0 & 1) != 0);

                self.square_pulse_2
                    .borrow_mut()
                    .length_counter_mut()
                    .set_enabled((data >> 1 & 1) != 0);

                self.triangle
                    .borrow_mut()
                    .length_counter_mut()
                    .set_enabled((data >> 2 & 1) != 0);

                self.noise
                    .borrow_mut()
                    .length_counter_mut()
                    .set_enabled((data >> 3 & 1) != 0);

                let mut dmc = self.dmc.borrow_mut();
                if data >> 4 & 1 == 0 {
                    dmc.clear_sample_remaining_bytes_and_silence();
                } else {
                    if !dmc.sample_remaining_bytes_more_than_0() {
                        dmc.restart_sample();
                    }
                }

                dmc.clear_interrupt_flag();
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
        if let Some(ref player) = self.player {
            player.play();
        }
    }

    pub fn pause(&self) {
        if let Some(ref player) = self.player {
            player.pause();
        }
    }

    fn length_counter_decrement<S: APUChannel>(channel: &mut Rc<RefCell<LengthCountedChannel<S>>>) {
        channel.borrow_mut().length_counter_mut().decrement();
    }

    fn envelope_clock<S: EnvelopedChannel>(channel: &mut Rc<RefCell<S>>) {
        channel.borrow_mut().clock_envlope();
    }

    fn timer_clock<S: TimedAPUChannel>(channel: &mut Rc<RefCell<S>>) {
        channel.borrow_mut().timer_clock();
    }

    fn triangle_linear_counter_clock(&mut self) {
        self.triangle
            .borrow_mut()
            .channel_mut()
            .clock_linear_counter();
    }

    fn square_sweeper_clock(channel: &mut Rc<RefCell<LengthCountedChannel<SquarePulse>>>) {
        channel.borrow_mut().channel_mut().clock_sweeper();
    }

    fn generate_quarter_frame_clock(&mut self) {
        Self::envelope_clock(&mut self.square_pulse_1);
        Self::envelope_clock(&mut self.square_pulse_2);
        Self::envelope_clock(&mut self.noise);
        self.triangle_linear_counter_clock();
    }

    fn generate_half_frame_clock(&mut self) {
        Self::length_counter_decrement(&mut self.square_pulse_1);
        Self::square_sweeper_clock(&mut self.square_pulse_1);
        Self::length_counter_decrement(&mut self.square_pulse_2);
        Self::square_sweeper_clock(&mut self.square_pulse_2);
        Self::length_counter_decrement(&mut self.triangle);
        Self::length_counter_decrement(&mut self.noise);
    }

    pub fn update_apu_freq(&mut self, apu_freq: f64) {
        self.apu_freq = apu_freq;
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

        // after how many apu clocks a sample should be recorded
        let samples_every_n_apu_clock = (self.apu_freq / (crate::SAMPLE_RATE as f64)) - self.offset;

        if self.cycle % 300 == 0 {
            if let Ok(mut buffered_channel) = self.buffered_channel.lock() {
                let change = if buffered_channel.get_is_overusing() {
                    0.001
                } else if buffered_channel.get_is_underusing() {
                    -0.0002
                } else {
                    0.
                };

                self.offset += change;
                buffered_channel.clear_using_flags();
            }
        }

        self.sample_counter += 1.0;
        if self.sample_counter >= samples_every_n_apu_clock {
            let output = self.mixer.get_output();

            self.buffered_channel.lock().unwrap().recored_sample(output);

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
        let dmc_irq_request = self.dmc.borrow().is_irq_change_requested();

        self.request_interrupt_flag_change.get() || dmc_irq_request
    }

    fn irq_pin_state(&self) -> bool {
        let dmc_irq = self.dmc.borrow().get_irq_pin_state();

        self.interrupt_flag.get() || dmc_irq
    }

    fn clear_irq_request_pin(&mut self) {
        self.request_interrupt_flag_change.set(false);

        self.dmc.borrow_mut().clear_irq_request_pin();
    }
}

impl APUCPUConnection for APU2A03 {
    fn request_dmc_reader_read(&self) -> Option<u16> {
        self.dmc.borrow().request_dmc_reader_read()
    }

    fn submit_buffer_byte(&mut self, byte: u8) {
        self.dmc.borrow_mut().submit_buffer_byte(byte);
    }
}
