#[macro_use]
mod common;
mod apu2a03;
mod cartridge;
mod controller;
mod cpu6502;
mod display;
#[cfg(feature = "frontend_misc")]
pub mod misc;
mod ppu2c02;

#[cfg(test)]
mod tests;

pub mod nes;

pub mod nes_controller {
    pub use super::controller::{StandardNESControllerState, StandardNESKey};
}
pub mod nes_display {
    pub use super::display::{Color, TV_BUFFER_SIZE, TV_HEIGHT, TV_WIDTH};
}
pub mod nes_audio {
    pub use super::apu2a03::SAMPLE_RATE;
}
