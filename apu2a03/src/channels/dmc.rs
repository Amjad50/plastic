use crate::tone_source::{APUChannel, TimedAPUChannel};

const DMC_PERIOD_RATES_NTSC: [u16; 0x10] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

pub struct Dmc {
    period: u16,
    current_timer: u16,

    samples_address: u16,
    samples_length: u16,

    samples_address_counter: u16,
    samples_remaining_bytes_counter: u16,

    sample_buffer: u8,
    sample_buffer_empty: bool,

    output_shift_register: u8,
    shifter_remaining_bits_counter: u8,
    output_silence_flag: bool,
    silence_on_next_empty: bool,
    output_level: u8,

    loop_flag: bool,

    irq_enabled_flag: bool,
    irq_pin_state: bool,
    is_irq_change_requested: bool,
}

impl Dmc {
    pub fn new() -> Self {
        Self {
            period: 0,
            current_timer: 0,

            samples_address: 0,
            samples_length: 0,

            samples_address_counter: 0,
            samples_remaining_bytes_counter: 0,

            sample_buffer: 0,
            sample_buffer_empty: true,

            output_shift_register: 0,
            shifter_remaining_bits_counter: 0,
            output_silence_flag: false,
            silence_on_next_empty: false,
            output_level: 0,

            loop_flag: false,

            irq_enabled_flag: false,
            irq_pin_state: false,
            is_irq_change_requested: false,
        }
    }

    pub(crate) fn set_irq_enabled_flag(&mut self, flag: bool) {
        self.irq_enabled_flag = flag;

        if !flag {
            self.irq_pin_state = false;
            self.is_irq_change_requested = true;
        }
    }

    pub(crate) fn set_loop_flag(&mut self, flag: bool) {
        self.loop_flag = flag;
    }

    pub(crate) fn set_rate_index(&mut self, rate_index: u8) {
        // since the table is in CPU clocks, /2 to make it in APU clocks periods
        self.period = DMC_PERIOD_RATES_NTSC[rate_index as usize & 0xF] / 2;
    }

    pub(crate) fn set_direct_output_level_load(&mut self, output_level: u8) {
        self.output_level = output_level & 0x7F;
    }

    pub(crate) fn set_samples_address(&mut self, address: u8) {
        self.samples_address = 0xC000 | ((address as u16) << 6);
        // self.samples_address_counter = self.samples_address;
    }

    pub(crate) fn set_samples_length(&mut self, length: u8) {
        self.samples_length = ((length as u16) << 4) + 1;
    }

    pub(crate) fn request_dmc_reader_read(&self) -> Option<u16> {
        if self.sample_buffer_empty && self.samples_remaining_bytes_counter != 0 {
            Some(self.samples_address_counter)
        } else {
            None
        }
    }

    pub(crate) fn submit_buffer_byte(&mut self, byte: u8) {
        assert!(self.sample_buffer_empty);

        self.sample_buffer = byte;
        self.sample_buffer_empty = false;

        self.samples_address_counter = if self.samples_address_counter == 0xFFFF {
            // overflow
            0x8000
        } else {
            self.samples_address_counter + 1
        };

        self.samples_remaining_bytes_counter =
            self.samples_remaining_bytes_counter.saturating_sub(1);

        if self.samples_remaining_bytes_counter == 0 {
            if self.loop_flag {
                self.restart_sample();
            } else if self.irq_enabled_flag {
                self.irq_pin_state = true;
                self.is_irq_change_requested = true;
            }
        }
    }

    pub(crate) fn sample_remaining_bytes_more_than_0(&self) -> bool {
        self.samples_remaining_bytes_counter > 0
    }

    pub(crate) fn get_irq_pin_state(&self) -> bool {
        self.irq_pin_state
    }

    pub(crate) fn is_irq_change_requested(&self) -> bool {
        self.is_irq_change_requested
    }

    pub(crate) fn clear_irq_request_pin(&mut self) {
        self.is_irq_change_requested = false;
    }

    pub(crate) fn clear_interrupt_flag(&mut self) {
        self.irq_pin_state = false;
        self.is_irq_change_requested = true;
    }

    pub(crate) fn clear_sample_remaining_bytes_and_silence(&mut self) {
        self.samples_remaining_bytes_counter = 0;
        self.silence_on_next_empty = true;
    }

    pub(crate) fn restart_sample(&mut self) {
        self.samples_remaining_bytes_counter = self.samples_length;
        self.samples_address_counter = self.samples_address;
    }
}

impl APUChannel for Dmc {
    fn get_output(&mut self) -> f32 {
        (self.output_level & 0x7F) as f32
    }
}

impl TimedAPUChannel for Dmc {
    fn timer_clock(&mut self) {
        if self.current_timer == 0 {
            if !self.output_silence_flag {
                if self.output_shift_register & 1 != 0 && self.output_level <= 125 {
                    self.output_level += 2;
                } else if self.output_shift_register & 1 == 0 && self.output_level >= 2 {
                    self.output_level -= 2;
                }
            }

            self.output_shift_register >>= 1;
            self.shifter_remaining_bits_counter -= 1;

            if self.shifter_remaining_bits_counter == 0 {
                self.shifter_remaining_bits_counter = 8;

                if self.sample_buffer_empty {
                    self.output_silence_flag = true;
                } else {
                    self.output_silence_flag = false;
                    self.output_shift_register = self.sample_buffer;
                    self.sample_buffer_empty = true;

                    if self.silence_on_next_empty {
                        self.output_silence_flag = true;
                        self.silence_on_next_empty = false;
                    }
                }
            }

            self.current_timer = self.period;
        } else {
            self.current_timer -= 1;
        }
    }
}
