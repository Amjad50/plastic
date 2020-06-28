use crate::color::Color;
use std::sync::{Arc, Mutex};

pub struct TV {
    // used sync, because this will be shared by the PPU and the Displaying
    // process, which would run in two threads to maximize performance
    pixels: Arc<Mutex<Vec<u8>>>,
    width: u32,
    height: u32,
}

impl TV {
    pub fn new(width: u32, height: u32) -> Self {
        let mut vec = Vec::new();

        // I'll be using SFML, which has the colors in 4 u8 elements in
        // the array
        vec.resize((width * height * 4) as usize, 0);

        Self {
            pixels: Arc::new(Mutex::new(vec)),
            width,
            height,
        }
    }

    // this will be transfered to another thread
    pub fn get_image_clone(&self) -> Arc<Mutex<Vec<u8>>> {
        self.pixels.clone()
    }

    pub fn set_pixel(&self, x: u32, y: u32, color: &Color) {
        let mut pixels = self
            .pixels
            .lock()
            .expect("Error in retrieving pixels lock for TV");

        let index = (y * self.width + x) as usize * 4;

        pixels[index + 0] = color.r;
        pixels[index + 1] = color.g;
        pixels[index + 2] = color.b;
        pixels[index + 3] = 0xFF; // full opacity
    }
}
