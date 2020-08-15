use crate::tone_source::APUChannel;

pub struct EnvelopeGenerator {
    start_flag: bool,
    loop_flag: bool,

    /// also used for constant volume
    divider_reload_value: u8,
    divider_counter: u8,

    use_constant_volume: bool,

    decay_level: u8,
}

impl EnvelopeGenerator {
    pub fn new() -> Self {
        Self {
            start_flag: false,
            loop_flag: false,
            divider_reload_value: 0,
            divider_counter: 0,
            use_constant_volume: false,
            decay_level: 0,
        }
    }

    pub(crate) fn set_volume(&mut self, vol: u8, use_vol: bool) {
        assert!(vol < 0x10);

        self.divider_reload_value = vol;
        self.use_constant_volume = use_vol;
    }

    pub(crate) fn set_start_flag(&mut self, start_flag: bool) {
        self.start_flag = start_flag;
    }

    pub(crate) fn set_loop_flag(&mut self, loop_flag: bool) {
        self.loop_flag = loop_flag;
    }

    pub(crate) fn clock(&mut self) {
        // When clocked by the frame counter, one of two actions occurs: if the
        // start flag is clear, the divider is clocked, otherwise the start
        // flag is cleared, the decay level counter is loaded with 15, and
        // the divider's period is immediately reloaded.
        if self.start_flag {
            self.start_flag = false;
            self.decay_level = 15;
            self.divider_counter = self.divider_reload_value;
        } else {
            // When the divider is clocked while at 0, it is loaded with V
            // and clocks the decay level counter. Then one of two actions
            // occurs: If the counter is non-zero, it is decremented, otherwise
            // if the loop flag is set, the decay level counter is loaded with 15.
            if self.divider_counter == 0 {
                self.divider_counter = self.divider_reload_value;

                self.decay_level = if self.loop_flag {
                    15
                } else {
                    self.decay_level.saturating_sub(1)
                };
            } else {
                self.divider_counter = self.divider_counter.saturating_sub(1);
            }
        }
    }

    /// return the volume 0 - 1
    pub(crate) fn get_current_volume(&mut self) -> f32 {
        if self.use_constant_volume {
            self.divider_reload_value as f32 / 0xF as f32
        } else {
            self.decay_level as f32 / 0xF as f32
        }
    }
}

pub trait EnvelopedChannel: APUChannel {
    fn clock_envlope(&mut self);
    fn envelope_generator_mut(&mut self) -> &mut EnvelopeGenerator;
}
