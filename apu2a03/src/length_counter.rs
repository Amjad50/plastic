use crate::tone_source::APUChannel;
use rodio::Sample;

const LEGNTH_COUNTER_TABLE: [u8; 0x20] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

pub struct LengthCounter {
    counter: u8,
    enabled: bool,
    halt: bool,
}

impl LengthCounter {
    pub fn empty() -> Self {
        Self {
            counter: 0,
            enabled: false,
            halt: false,
        }
    }

    /// using the value `index` as index to the table
    ///      |  0   1   2   3   4   5   6   7    8   9   A   B   C   D   E   F
    /// -----+----------------------------------------------------------------
    /// 00-0F  10,254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
    /// 10-1F  12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30
    pub(crate) fn reload_counter(&mut self, index: u8) {
        assert!(index <= 0x1F);

        self.counter = LEGNTH_COUNTER_TABLE[index as usize];
    }

    /// decrement if appropriate, it will not decrement when:
    /// The length counter is 0, or The halt flag is set, and it will
    /// set to 0 constantly if `enabled` is false
    pub(crate) fn decrement(&mut self) {
        self.counter = if self.enabled {
            // `saturating_sub` will stop at 0 instead of overflowing
            self.counter.saturating_sub((!self.halt) as u8)
        } else {
            0
        }
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        // silence immediately
        if !enabled {
            self.counter = 0;
        }
    }

    pub(crate) fn set_halt(&mut self, halt: bool) {
        self.halt = halt;
    }

    pub(crate) fn counter(&self) -> u8 {
        self.counter
    }
}

pub struct LengthCountedChannel<C>
where
    C: APUChannel,
    C::Item: Sample,
{
    length_counter: LengthCounter,
    channel: C,
}

impl<C> LengthCountedChannel<C>
where
    C: APUChannel,
    C::Item: Sample,
{
    pub fn new(channel: C) -> Self {
        Self {
            length_counter: LengthCounter::empty(),
            channel,
        }
    }

    pub(crate) fn length_counter(&self) -> &LengthCounter {
        &self.length_counter
    }

    pub(crate) fn length_counter_mut(&mut self) -> &mut LengthCounter {
        &mut self.length_counter
    }

    pub(crate) fn channel(&self) -> &C {
        &self.channel
    }

    pub(crate) fn channel_mut(&mut self) -> &mut C {
        &mut self.channel
    }
}

impl<C> Iterator for LengthCountedChannel<C>
where
    C: APUChannel,
    C::Item: Sample,
{
    type Item = C::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.length_counter.counter == 0 {
            Some(Self::Item::zero_value())
        } else {
            self.channel.next()
        }
    }
}

impl<C> APUChannel for LengthCountedChannel<C>
where
    C: APUChannel,
    C::Item: Sample,
{
}
