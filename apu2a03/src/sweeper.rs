use crate::channels::SquarePulse;
use crate::envelope::EnvelopeGenerator;
use crate::length_counter::LengthCountedChannel;
use std::sync::{Arc, Mutex};

pub struct Sweeper {
    enabled: bool,
    divider_period_reload_value: u8,
    divider_period_counter: u8,
    negative: bool,
    shift_count: u8,

    // FIXME: use more generic approach
    square_channel: Arc<Mutex<LengthCountedChannel<EnvelopeGenerator<SquarePulse>>>>,
}

impl Sweeper {
    pub fn new(
        square_channel: Arc<Mutex<LengthCountedChannel<EnvelopeGenerator<SquarePulse>>>>,
    ) -> Self {
        Self {
            enabled: false,
            divider_period_reload_value: 0,
            divider_period_counter: 0,
            negative: false,
            shift_count: 0,

            square_channel,
        }
    }

    pub(crate) fn set_from_data_byte(&mut self, data: u8) {
        self.enabled = data & 0x80 != 0;
        self.divider_period_reload_value = (data >> 4) & 0b111;
        self.negative = data & 0x08 != 0;
        self.shift_count = data & 0b111;
    }

    pub(crate) fn clock(&mut self) {
        if self.enabled {
            if self.divider_period_counter == 0 {
                if let Ok(mut channel) = self.square_channel.lock() {
                    let current_period = channel.channel().channel().get_period();
                    let change_amount = current_period >> self.shift_count;

                    let target_period = if self.negative {
                        // TODO: handle the differences between sqr1 and sqr2
                        // sqr1 adds the ones' complement (−c − 1).
                        // Making 20 negative produces a change amount of −21.
                        // sqr2 adds the two's complement (−c).
                        // Making 20 negative produces a change amount of −20.
                        current_period.saturating_sub(change_amount)
                    } else {
                        current_period.saturating_add(change_amount)
                    };

                    if target_period > 0x7FF || target_period < 8 {
                        // sweep muting
                        channel.channel_mut().channel_mut().set_muted(true);
                    } else {
                        channel
                            .channel_mut()
                            .channel_mut()
                            .set_period(target_period);
                        channel.channel_mut().channel_mut().set_muted(false);
                    }
                }

                self.divider_period_counter = self.divider_period_reload_value;
            } else {
                self.divider_period_counter -= 1;
            }
        }
    }
}
