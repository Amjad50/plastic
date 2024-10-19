use super::color::Color;

pub const TV_WIDTH: usize = 256;
pub const TV_HEIGHT: usize = 240;
const COLOR_BYTES_LEN: usize = 3;
pub const TV_BUFFER_SIZE: usize = TV_WIDTH * TV_HEIGHT * COLOR_BYTES_LEN;

pub struct TV {
    /// Current pixel buffer ready for display.
    pixels_to_display: Box<[u8; TV_BUFFER_SIZE]>,

    /// A temporary buffer to holds the screen state while the PPU is drawing
    /// in the current frame
    building_pixels: Box<[Color; TV_WIDTH * TV_HEIGHT]>,
}

impl TV {
    pub fn new() -> Self {
        Self {
            pixels_to_display: Box::new([0; TV_BUFFER_SIZE]),
            building_pixels: Box::new([color!(0, 0, 0); TV_WIDTH * TV_HEIGHT]),
        }
    }

    /// update the pixel of the temporary buffer [`building_pixels`]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: &Color) {
        let index = y as usize * TV_WIDTH + x as usize;
        self.building_pixels[index] = *color;
    }

    /// the PPU must call this at the end of the frame, maybe around `VBLANK`
    /// to tell the screen to copy and translate the [`Color`] data into the
    /// [`Arc`] shared screen buffer
    pub fn signal_end_of_frame(&mut self) {
        for (result, color) in self
            .pixels_to_display
            .chunks_exact_mut(COLOR_BYTES_LEN)
            .zip(self.building_pixels.iter())
        {
            result[0..COLOR_BYTES_LEN].copy_from_slice(&[color.r, color.g, color.b]);
        }
    }

    /// resets and zero all buffers
    pub fn reset(&mut self) {
        for i in self.pixels_to_display.iter_mut() {
            *i = 0;
        }

        for i in self.building_pixels.as_mut() {
            *i = color!(0, 0, 0);
        }
    }

    pub fn display_pixel_buffer(&self) -> &[u8] {
        self.pixels_to_display.as_ref()
    }
}
