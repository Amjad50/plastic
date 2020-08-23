use nes_ui_base::{
    nes::{TV_HEIGHT, TV_WIDTH},
    nes_controller::{StandardNESControllerState, StandardNESKey},
    nes_display::Color as NESColor,
    UiEvent, UiProvider,
};
use std::sync::{atomic::AtomicBool, atomic::Ordering, mpsc::Sender, Arc, Mutex};

use native_windows_derive as nwd;
use native_windows_gui as nwg;

use nwd::NwgUi;
use nwg::{keys, EventData, ExternCanvas, NativeUi, Timer, Window};
use winapi::um::{
    wingdi::{
        BitBlt, CreateBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject,
        SelectObject, SetStretchBltMode, StretchBlt, COLORONCOLOR, RGB, SRCCOPY,
    },
    winuser::FillRect,
};

#[derive(NwgUi)]
pub struct ProviderApp {
    #[nwg_control(
        size: (TV_WIDTH as i32 * 3, TV_HEIGHT as i32 * 3),
        title: "Plastic",
        flags: "WINDOW|VISIBLE|MAIN_WINDOW|RESIZABLE"
    )]
    #[nwg_events(
        OnWindowClose: [nwg::stop_thread_dispatch()],
        // handle all cases for resizing [not good] :(
        OnInit:           [ProviderApp::window_resize(SELF, CTRL)],
        OnResize:         [ProviderApp::window_resize(SELF, CTRL)],
        OnWindowMaximize: [ProviderApp::window_resize(SELF, CTRL)],
        OnWindowMinimize: [ProviderApp::window_resize(SELF, CTRL)],

        OnKeyPress:   [ProviderApp::key_pressed(SELF, EVT_DATA)],
        OnKeyRelease: [ProviderApp::key_released(SELF, EVT_DATA)],
    )]
    window: Window,

    #[nwg_control(parent: Some(&data.window), position: (0, 0), size: (280, 280))]
    #[nwg_events(OnPaint: [ProviderApp::paint(SELF, CTRL, EVT_DATA)])]
    canvas: ExternCanvas,

    #[nwg_control(parent: window, interval: 1000/60, stopped: false)]
    #[nwg_events(OnTimerTick: [ProviderApp::timer_tick(SELF)])]
    timer: Timer,

    ui_to_nes_sender: Sender<UiEvent>,
    image: Arc<Mutex<Vec<u8>>>,
    ctrl_state: Arc<Mutex<StandardNESControllerState>>,
}

impl ProviderApp {
    fn initial_state(
        ui_to_nes_sender: Sender<UiEvent>,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<StandardNESControllerState>>,
    ) -> Self {
        Self {
            window: Default::default(),
            canvas: Default::default(),
            timer: Default::default(),
            ui_to_nes_sender,
            image,
            ctrl_state,
        }
    }
}

impl ProviderApp {
    fn window_resize(&self, ctrl: &Window) {
        self.canvas.set_size(ctrl.size().0, ctrl.size().1);
    }

    fn key_pressed(&self, data: &EventData) {
        let mut ctrl = self.ctrl_state.lock().unwrap();

        match data.on_key() {
            keys::_J => ctrl.press(StandardNESKey::B),
            keys::_K => ctrl.press(StandardNESKey::A),
            keys::_U => ctrl.press(StandardNESKey::Select),
            keys::_I => ctrl.press(StandardNESKey::Start),
            keys::_W => ctrl.press(StandardNESKey::Up),
            keys::_S => ctrl.press(StandardNESKey::Down),
            keys::_A => ctrl.press(StandardNESKey::Left),
            keys::_D => ctrl.press(StandardNESKey::Right),
            _ => {}
        }
    }

    fn key_released(&self, data: &EventData) {
        let mut ctrl = self.ctrl_state.lock().unwrap();

        match data.on_key() {
            keys::_J => ctrl.release(StandardNESKey::B),
            keys::_K => ctrl.release(StandardNESKey::A),
            keys::_U => ctrl.release(StandardNESKey::Select),
            keys::_I => ctrl.release(StandardNESKey::Start),
            keys::_W => ctrl.release(StandardNESKey::Up),
            keys::_S => ctrl.release(StandardNESKey::Down),
            keys::_A => ctrl.release(StandardNESKey::Left),
            keys::_D => ctrl.release(StandardNESKey::Right),
            _ => {}
        }
    }

    fn paint(&self, ctrl: &ExternCanvas, data: &EventData) {
        let paint = data.on_paint();
        let ps = paint.begin_paint();

        let hdc = ps.hdc;
        let rc = &ps.rcPaint;

        let data: *const u8 = self.image.lock().unwrap().as_ptr();

        // All/Most functions from the winapi are unsafe, so ya
        unsafe {
            let brush: *mut _ = &mut CreateSolidBrush(RGB(0, 0, 0));
            FillRect(hdc, rc, brush as _);

            let bitmap = CreateBitmap(TV_WIDTH as i32, TV_HEIGHT as i32, 1, 32, data as _);

            // used for setting the bitmap and scaling it before moving it to
            // the original hdc
            let hdctmp = CreateCompatibleDC(hdc);

            let hbmold = SelectObject(hdctmp, bitmap as _);

            BitBlt(
                hdctmp,
                0,
                0,
                TV_WIDTH as i32,
                TV_HEIGHT as i32,
                hdctmp,
                0,
                0,
                SRCCOPY,
            );

            SetStretchBltMode(hdctmp, COLORONCOLOR);

            let area_width = ctrl.size().0 as f64;
            let area_height = ctrl.size().1 as f64;

            // Got from the GTK UI code
            let scale_width = area_width / TV_WIDTH as f64;
            let scale_height = area_height / TV_HEIGHT as f64;

            let scale_smallest;
            let mut top = 0.;
            let mut left = 0.;

            if scale_width > scale_height {
                scale_smallest = scale_height;
                left = (area_width - (TV_WIDTH as f64 * scale_smallest)) / 2.;
            } else {
                scale_smallest = scale_width;
                top = (area_height - (TV_HEIGHT as f64 * scale_smallest)) / 2.;
            };

            let new_width = (TV_WIDTH as f64 * scale_smallest) as i32;
            let new_height = (TV_HEIGHT as f64 * scale_smallest) as i32;

            StretchBlt(
                hdc,
                left as i32,
                top as i32,
                new_width,
                new_height,
                hdctmp,
                0,
                0,
                TV_WIDTH as i32,
                TV_HEIGHT as i32,
                SRCCOPY,
            );

            SelectObject(hdctmp, hbmold);
            DeleteDC(hdctmp);
            DeleteObject(bitmap as _);
        }

        paint.end_paint(&ps);
    }

    fn timer_tick(&self) {
        self.canvas.invalidate();
    }
}

pub struct NwgProvider {
    paused: Arc<AtomicBool>,
}

impl NwgProvider {
    pub fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl UiProvider for NwgProvider {
    fn get_tv_color_converter() -> fn(&NESColor) -> [u8; 4] {
        |color| [color.b, color.g, color.r, 0xFF]
    }

    fn run_ui_loop(
        &mut self,
        ui_to_nes_sender: Sender<UiEvent>,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<StandardNESControllerState>>,
    ) {
        nwg::init().expect("Failed to init Native Windows GUI");
        nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

        let _app = ProviderApp::build_ui(ProviderApp::initial_state(
            ui_to_nes_sender,
            image,
            ctrl_state,
        ))
        .expect("Failed to build UI");

        nwg::dispatch_thread_events();
    }
}
