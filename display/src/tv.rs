use crate::color::Color;
use std::sync::{Arc, Mutex};

pub const TV_WIDTH: usize = 256;
pub const TV_HEIGHT: usize = 240;
const COLOR_BYTES_LEN: usize = 4;
pub const TV_BUFFER_SIZE: usize = TV_WIDTH * TV_HEIGHT * COLOR_BYTES_LEN;

pub struct TV {
    /// this buffer is being read by the UI provider, and written to by the PPU,
    /// but for performance, we only update it once per frame, and the current
    /// being drawn is being updated in [`building_pixels`]
    pixels_to_display: Arc<Mutex<Vec<u8>>>,

    /// A temporary buffer to holds the screen state while the PPU is drawing
    /// in the current frame
    building_pixels: [Color; TV_WIDTH * TV_HEIGHT],

    /// A function to convert from [`Color`] to 4 byte value, which is used by
    /// the UI provider
    pixels_handler: fn(&Color) -> [u8; 4],
}

impl TV {
    pub fn new(pixels_handler: fn(&Color) -> [u8; COLOR_BYTES_LEN]) -> Self {
        Self {
            pixels_to_display: Arc::new(Mutex::new(vec![0; TV_BUFFER_SIZE])),
            building_pixels: [color!(0, 0, 0); TV_WIDTH * TV_HEIGHT],
            pixels_handler,
        }
    }

    /// this will be transfered to another thread
    pub fn get_image_clone(&self) -> Arc<Mutex<Vec<u8>>> {
        self.pixels_to_display.clone()
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
        if let Ok(mut buffer) = self.pixels_to_display.lock() {
            for (result, color) in buffer
                .chunks_exact_mut(COLOR_BYTES_LEN)
                .zip(self.building_pixels.iter())
            {
                result[0..4].copy_from_slice(&(self.pixels_handler)(color));
            }
        }
    }

    /// resets and zero all buffers
    pub fn reset(&mut self) {
        if let Ok(mut buffer) = self.pixels_to_display.lock() {
            for i in buffer.iter_mut() {
                *i = 0;
            }
        }

        for i in &mut self.building_pixels {
            *i = color!(0, 0, 0);
        }
    }
}
