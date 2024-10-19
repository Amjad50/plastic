//! # Plastic NES Core
//!
//! This is the core of the Plastic NES emulator. It contains the CPU, PPU, APU, and other components
//! and can be used standalone to run/emulate NES games if you want to handle your own input/output.
//!
//! A very simple example of how to use this crate:
//! ```no_run
//! use plastic_core::NES;
//! # fn display(_: &[u8]) {}
//! # fn play_audio(_: &[f32]) {}
//!
//! fn main() {
//!    let mut nes = NES::new("path/to/rom-file.nes").unwrap();
//!    
//!    loop {
//!        nes.clock_for_frame();
//!
//!        let pixel_buffer = nes.pixel_buffer();
//!        display(&pixel_buffer);
//!
//!        let audio_buffer = nes.audio_buffer();
//!        play_audio(&audio_buffer);
//!    }
//! }
//! ```
//! In the

#[macro_use]
mod common;
mod apu2a03;
mod cartridge;
mod controller;
mod cpu6502;
mod display;
#[cfg(feature = "frontend_misc")]
pub mod misc;
mod nes;
mod ppu2c02;

#[cfg(test)]
mod tests;

pub use cartridge::CartridgeError;
pub use common::save_state::SaveError;
pub use controller::NESKey;
pub use nes::NES;

/// Structures used when interacting with the CPU, see also [`NES::clock`][NES::clock]
pub mod cpu {
    pub use super::cpu6502::CPURunState;
}

/// Helper variables related to handling pixel buffers from the emulator
pub mod nes_display {
    pub use super::display::{COLOR_BYTES_LEN, TV_BUFFER_SIZE, TV_HEIGHT, TV_WIDTH};
}
/// Helper variables related to handling audio buffers from the emulator
pub mod nes_audio {
    pub use super::apu2a03::SAMPLE_RATE;
}
