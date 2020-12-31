#[macro_use]
mod color;
mod tv;

pub use color::Color;
pub use color::COLORS;
pub use tv::{TV, TV_BUFFER_SIZE, TV_HEIGHT, TV_WIDTH};
