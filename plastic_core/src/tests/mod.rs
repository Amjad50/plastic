use crate::cartridge::CartridgeError;
use crate::common::{Bus, Device};
use crate::cpu6502::{CPUBusTrait, CPURunState};
use crate::display::{COLORS, TV_WIDTH};
use crate::nes::NES;
use std::{
    convert::From,
    error::Error,
    fmt::{Debug, Display, Formatter, Result as fmtResult},
};

mod blargg_tests;

pub enum TestError {
    CartridgeError(CartridgeError),
    ResultError(u8),
}

impl TestError {
    fn get_message(&self) -> String {
        match self {
            Self::CartridgeError(err) => format!("CartridgeError: {}", err),
            Self::ResultError(code) => format!("ResultError: test failed with code {}", code),
        }
    }
}

impl Error for TestError {}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.get_message())
    }
}

impl Debug for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmtResult {
        write!(f, "{}", self.get_message())
    }
}

impl From<CartridgeError> for TestError {
    fn from(from: CartridgeError) -> Self {
        Self::CartridgeError(from)
    }
}

pub struct NesTester {
    nes: NES,
}

impl NesTester {
    pub fn new(filename: &str) -> Result<Self, CartridgeError> {
        let nes = NES::new(filename)?;

        Ok(Self { nes })
    }

    pub fn cpu_read_address(&self, address: u16) -> u8 {
        self.nes.cpu_bus().read(address)
    }

    pub fn ppu_read_address(&self, address: u16) -> u8 {
        self.nes.ppu_bus().read(address, Device::Ppu)
    }

    pub fn pixel_buffer(&self) -> &[u8] {
        self.nes.pixel_buffer()
    }

    pub fn clock(&mut self) -> CPURunState {
        self.nes.clock().unwrap()
    }

    pub fn clock_until_infinite_loop(&mut self) {
        loop {
            if let CPURunState::InfiniteLoop(_) = self.clock() {
                break;
            }
        }
    }

    pub fn clock_until_nmi(&mut self) {
        loop {
            if let CPURunState::StartingInterrupt = self.clock() {
                break;
            }
        }
    }

    /// loop until the memory at `address` does not equal to `data`
    pub fn clock_until_memory_neq(&mut self, address: u16, data: u8) {
        loop {
            self.clock();

            if self.cpu_read_address(address) != data {
                break;
            }
        }
    }

    /// after each CPU clock (3 PPU clocks), check if the pixel in `x, y`
    /// match the color specified `color_code`, if match, then return
    ///
    /// this check is done manually now, not sure if it should be added
    /// to `display::TV` or not
    pub fn clock_until_pixel_appears(&mut self, x: u32, y: u32, color_code: u8) {
        loop {
            self.clock();

            let index = (y * TV_WIDTH as u32 + x) as usize * 3;

            let pixel_buffer = self.pixel_buffer();
            let color = &COLORS[color_code as usize];

            if pixel_buffer[index] == color.r
                && pixel_buffer[index + 1] == color.g
                && pixel_buffer[index + 2] == color.b
            {
                break;
            }
        }
    }
}
