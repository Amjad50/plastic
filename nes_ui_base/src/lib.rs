pub mod nes;

pub mod nes_controller {
    pub use controller::{StandardNESControllerState, StandardNESKey};
}
pub mod nes_display {
    pub use display::Color;
}

use std::sync::{Arc, Mutex};

pub trait UiProvider {
    // TODO: for now only supported are 32-bit size pixel data, maybe later we can
    //  support more
    /// get the color converter for this UI provider, the reason this is good to have
    /// is performance, as some UI for example use pixel data in form (RGBA) or (ARGB)
    /// so this function will be called on every pixel set by the PPU in the time
    /// it is set instead of doing it in the UI thread for the whole frame
    ///
    fn get_tv_color_converter() -> fn(&display::Color) -> [u8; 4];

    /// initialize and run the UI loop,
    /// this method will be called in another thread, so make sure it does not
    /// return unless the UI is closed, if this function returns, the emulation
    /// will stop and the emulator process will return
    ///
    /// [`image`] contains the raw image data
    /// [`ctrl_state`] is the controller state, the provider should change this
    /// based on buttons presses and releases
    fn run_ui_loop(
        &mut self,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<controller::StandardNESControllerState>>,
    );
}