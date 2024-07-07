use crate::common::{Bus, Device};
use bitflags::bitflags;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandardNESKey {
    A = 1 << 0,
    B = 1 << 1,
    Select = 1 << 2,
    Start = 1 << 3,
    Up = 1 << 4,
    Down = 1 << 5,
    Left = 1 << 6,
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
    pub fn press(&mut self, key: StandardNESKey) {
        self.insert(StandardNESControllerState::from_bits(key as u8).unwrap());
    }

    pub fn release(&mut self, key: StandardNESKey) {
        self.remove(StandardNESControllerState::from_bits(key as u8).unwrap());
    }
}

pub struct Controller {
    primary_state: Arc<Mutex<StandardNESControllerState>>,
    polled_state: Cell<u8>,

    polling: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            primary_state: Arc::new(Mutex::new(StandardNESControllerState::empty())),
            polled_state: Cell::new(0),

            polling: false,
        }
    }

    pub fn get_primary_controller_state(&self) -> Arc<Mutex<StandardNESControllerState>> {
        self.primary_state.clone()
    }
}

impl Bus for Controller {
    fn read(&self, _address: u16, _device: Device) -> u8 {
        // refresh polled here
        if self.polling {
            if let Ok(primary_state) = self.primary_state.lock() {
                self.polled_state.set(primary_state.bits);
            }
        }
        let result = self.polled_state.get() & 1;

        self.polled_state.set(self.polled_state.get() >> 1);

        result
    }

    fn write(&mut self, _address: u16, data: u8, _device: Device) {
        let new_polling = data & 1 == 1;

        // if the state changed, then refresh
        if self.polling ^ new_polling {
            if let Ok(primary_state) = self.primary_state.lock() {
                self.polled_state.set(primary_state.bits);
            }
        }

        self.polling = new_polling;
    }
}
