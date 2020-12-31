mod color;

// TODO: fix macro imports to `tv`
macro_rules! color {
    ($r:expr, $g:expr, $b:expr) => {
        Color {
            r: $r,
            g: $g,
            b: $b,
        }
    };
}

mod tv;

pub use color::Color;
pub use color::COLORS;
pub use tv::{TV, TV_BUFFER_SIZE, TV_HEIGHT, TV_WIDTH};
