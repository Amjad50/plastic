use crate::common::{Bus, Device};
use bitflags::bitflags;
use std::cell::Cell;

/// Represents the keys on an NES controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NESKey {
    /// The 'A' button.
    A = 1 << 0,
    /// The 'B' button.
    B = 1 << 1,
    /// The 'Select' button.
    Select = 1 << 2,
    /// The 'Start' button.
    Start = 1 << 3,
    /// The 'Up' directional button.
    Up = 1 << 4,
    /// The 'Down' directional button.
    Down = 1 << 5,
    /// The 'Left' directional button.
    Left = 1 << 6,
    /// The 'Right' directional button.
    Right = 1 << 7,
}

bitflags! {
   pub struct StandardNESControllerState : u8{
        const A = 1 << 0;
        const B = 1 << 1;
        const SELECT = 1 << 2;
        const START = 1 << 3;
        const UP = 1 << 4;
        const DOWN = 1 << 5;
        const LEFT = 1 << 6;
        const RIGHT = 1 << 7;
   }
}

impl StandardNESControllerState {
    fn press(&mut self, key: NESKey) {
        self.insert(StandardNESControllerState::from_bits(key as u8).unwrap());
    }

    fn release(&mut self, key: NESKey) {
        self.remove(StandardNESControllerState::from_bits(key as u8).unwrap());
    }

    pub fn set_controller_state(&mut self, key: NESKey, pressed: bool) {
        if pressed {
            self.press(key);
        } else {
            self.release(key);
        }
    }
}

pub struct Controller {
    primary_state: StandardNESControllerState,
    polled_state: Cell<u8>,

    polling: bool,
}

impl Controller {
    pub(crate) fn new() -> Self {
        Self {
            primary_state: StandardNESControllerState::empty(),
            polled_state: Cell::new(0),

            polling: false,
        }
    }

    pub fn set_controller_state(&mut self, key: NESKey, pressed: bool) {
        self.primary_state.set_controller_state(key, pressed);
    }
}

impl Bus for Controller {
    fn read(&self, _address: u16, _device: Device) -> u8 {
        // refresh polled here
        if self.polling {
            self.polled_state.set(self.primary_state.bits);
        }
        let result = self.polled_state.get() & 1;

        self.polled_state.set(self.polled_state.get() >> 1);

        result
    }

    fn write(&mut self, _address: u16, data: u8, _device: Device) {
        let new_polling = data & 1 == 1;

        // if the state changed, then refresh
        if self.polling ^ new_polling {
            self.polled_state.set(self.primary_state.bits);
        }

        self.polling = new_polling;
    }
}
