#[macro_use]
mod color;
mod tv;

pub use crate::color::Color;
pub use crate::color::COLORS;
pub use crate::tv::{TV, TV_BUFFER_SIZE, TV_HEIGHT, TV_WIDTH};
