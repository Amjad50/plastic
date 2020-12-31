#[macro_use]
mod bus;
mod mirroring;

pub mod interconnection;
pub mod save_state;

pub use bus::{Bus, Device};
pub use mirroring::{MirroringMode, MirroringProvider};

pub const CPU_FREQ: f64 = 1.789773 * 1E6;
