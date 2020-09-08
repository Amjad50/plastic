use nes_ui_base::{
    nes::{TV_HEIGHT, TV_WIDTH},
    nes_controller::{StandardNESControllerState, StandardNESKey},
    nes_display::Color as NESColor,
    BackendEvent, UiEvent, UiProvider,
};
use std::cell::Cell;
use std::sync::{
    atomic::AtomicBool,
    atomic::Ordering,
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

use native_windows_derive as nwd;
use native_windows_gui as nwg;

use nwd::NwgUi;
use nwg::{
    full_bind_event_handler, keys, ControlHandle, Event, EventData, ExternCanvas, FileDialog,
    FileDialogAction, Menu, MenuItem, MenuSeparator, NativeUi, Timer, Window,
};
use winapi::um::{
    wingdi::{
        BitBlt, CreateBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject,
        SelectObject, SetStretchBltMode, StretchBlt, COLORONCOLOR, RGB, SRCCOPY,
    },
    winuser::FillRect,
};

const NUMBER_OF_STATES: u8 = 10;

#[derive(NwgUi)]
pub struct ProviderApp {
    #[nwg_control(
        size: (TV_WIDTH as i32 * 3, TV_HEIGHT as i32 * 3),
        title: "Plastic",
        flags: "WINDOW|VISIBLE|MAIN_WINDOW|RESIZABLE",
        accept_files: true
    )]
    #[nwg_events(
        OnWindowClose: [nwg::stop_thread_dispatch()],
        // handle all cases for resizing [not good] :(
        OnInit:           [ProviderApp::init(SELF, CTRL)],
        OnResize:         [ProviderApp::window_resize(SELF, CTRL)],
        OnWindowMaximize: [ProviderApp::window_resize(SELF, CTRL)],
        OnWindowMinimize: [ProviderApp::window_resize(SELF, CTRL)],

        OnKeyPress:   [ProviderApp::key_pressed(SELF, EVT_DATA)],
        OnKeyRelease: [ProviderApp::key_released(SELF, EVT_DATA)],

        OnFileDrop: [ProviderApp::file_drop(SELF, EVT_DATA)],
    )]
    window: Window,

    #[nwg_control(parent: Some(&data.window), position: (0, 0), size: (280, 280))]
    #[nwg_events(OnPaint: [ProviderApp::paint(SELF, CTRL, EVT_DATA)])]
    canvas: ExternCanvas,

    #[nwg_control(parent: window, interval: 1000/60, stopped: false)]
    #[nwg_events(OnTimerTick: [ProviderApp::timer_tick(SELF)])]
    timer: Timer,

    #[nwg_control(parent: window, text: "&File", disabled: false, popup: false)]
    file_menu: Menu,

    #[nwg_control(parent: file_menu, text: "&Open", disabled: false, check: false)]
    #[nwg_events(OnMenuItemSelected: [ProviderApp::menu_action_open(SELF)])]
    file_menu_open_action: MenuItem,

    #[nwg_control(parent: file_menu)]
    _menu_separator_1: MenuSeparator,

    #[nwg_control(parent: file_menu, text: "&Save state", disabled: false, popup: false)]
    file_menu_save_state_menu: Menu,

    #[nwg_control(parent: file_menu, text: "&Load state", disabled: false, popup: false)]
    file_menu_load_state_menu: Menu,

    #[nwg_control(parent: file_menu)]
    _menu_separator_2: MenuSeparator,

    #[nwg_control(parent: file_menu, text: "&Quit", disabled: false, check: false)]
    #[nwg_events(OnMenuItemSelected: [ProviderApp::menu_action_quit(SELF)])]
    file_menu_quit_action: MenuItem,

    #[nwg_control(parent: window, text: "&Game", disabled: false, popup: false)]
    game_menu: Menu,

    #[nwg_control(parent: game_menu, text: "&Reset", disabled: false, check: false)]
    #[nwg_events(OnMenuItemSelected: [ProviderApp::menu_action_reset(SELF)])]
    game_menu_reset_action: MenuItem,

    #[nwg_control(parent: game_menu, text: "&Pause", disabled: false, check: false)]
    #[nwg_events(OnMenuItemSelected: [ProviderApp::menu_action_pause(SELF)])]
    game_menu_pause_action: MenuItem,

    #[nwg_control(parent: game_menu, text: "&Resume", disabled: false, check: false)]
    #[nwg_events(OnMenuItemSelected: [ProviderApp::menu_action_resume(SELF)])]
    game_menu_resume_action: MenuItem,

    ui_to_nes_sender: Sender<UiEvent>,
    nes_to_ui_receiver: Receiver<BackendEvent>,
    image: Arc<Mutex<Vec<u8>>>,
    ctrl_state: Arc<Mutex<StandardNESControllerState>>,

    paused: Arc<AtomicBool>,

    /// flag to indicate that the windows has been resized and we should clear
    /// the canvas, Fixes flickering
    need_clear_rect: Cell<bool>,
}

impl ProviderApp {
    fn initial_state(
        ui_to_nes_sender: Sender<UiEvent>,
        nes_to_ui_receiver: Receiver<BackendEvent>,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<StandardNESControllerState>>,
    ) -> Self {
        Self {
            window: Default::default(),
            canvas: Default::default(),
            timer: Default::default(),
            file_menu: Default::default(),
            file_menu_open_action: Default::default(),
            _menu_separator_1: Default::default(),
            file_menu_save_state_menu: Default::default(),
            file_menu_load_state_menu: Default::default(),
            _menu_separator_2: Default::default(),
            file_menu_quit_action: Default::default(),
            game_menu: Default::default(),
            game_menu_reset_action: Default::default(),
            game_menu_pause_action: Default::default(),
            game_menu_resume_action: Default::default(),

            ui_to_nes_sender,
            nes_to_ui_receiver,
            image,
            ctrl_state,
            paused: Arc::new(AtomicBool::new(false)),

            need_clear_rect: Cell::new(true),
        }
    }
}

impl ProviderApp {
    fn init(&self, ctrl: &Window) {
        for i in 1..=NUMBER_OF_STATES {
            let mut save_item = MenuItem::default();
            let mut load_item = MenuItem::default();

            let label = format!("&{} <empty>", i);
            MenuItem::builder()
                .text(&label)
                .parent(&self.file_menu_save_state_menu)
                .build(&mut save_item)
                .unwrap();

            MenuItem::builder()
                .text(&label)
                .parent(&self.file_menu_load_state_menu)
                .build(&mut load_item)
                .unwrap();

            // setup handlers
            {
                let ui_to_nes_sender = self.ui_to_nes_sender.clone();
                // TODO: for performance maybe its better to handle all items in one handler?
                full_bind_event_handler(&(&self.window).into(), move |event, _event_data, ctrl| {
                    if let Event::OnMenuItemSelected = event {
                        if &ctrl == &save_item {
                            ui_to_nes_sender.send(UiEvent::SaveState(i)).unwrap();
                        } else if &ctrl == &load_item {
                            ui_to_nes_sender.send(UiEvent::LoadState(i)).unwrap();
                        }
                    }
                });
            }
        }

        self.window_resize(ctrl);
    }

    fn window_resize(&self, ctrl: &Window) {
        self.canvas.set_size(ctrl.size().0, ctrl.size().1);

        // mark as dirty, and need to clear the canvas
        self.need_clear_rect.set(true);
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
            keys::ESCAPE => {
                if self.paused.load(Ordering::Relaxed) {
                    self.ui_to_nes_sender.send(UiEvent::Resume).unwrap();
                    self.paused.store(false, Ordering::Relaxed);
                } else {
                    self.ui_to_nes_sender.send(UiEvent::Pause).unwrap();
                    self.paused.store(true, Ordering::Relaxed);
                }
            }
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

    fn file_drop(&self, data: &EventData) {
        let drop_files = data.on_file_drop();

        if drop_files.len() > 0 {
            if let Some(filename) = drop_files.files().iter().find(|e| e.ends_with(".nes")) {
                self.ui_to_nes_sender
                    .send(UiEvent::LoadRom(filename.to_string()))
                    .unwrap();
            }
        }
    }

    fn menu_action_open(&self) {
        let mut file_dialog = Default::default();

        FileDialog::builder()
            .title("Select NES ROM")
            .action(FileDialogAction::Open)
            .multiselect(false)
            .filters("NES ROM(*.nes)")
            .build(&mut file_dialog)
            .unwrap();

        if file_dialog.run(Some(&self.window)) {
            if let Ok(filename) = file_dialog.get_selected_item() {
                if filename.ends_with(".nes") {
                    self.ui_to_nes_sender
                        .send(UiEvent::LoadRom(filename))
                        .unwrap();
                }
            }
        }
    }

    fn menu_action_quit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn menu_action_reset(&self) {
        self.ui_to_nes_sender.send(UiEvent::Reset).unwrap()
    }

    fn menu_action_pause(&self) {
        self.ui_to_nes_sender.send(UiEvent::Pause).unwrap();
        self.paused.store(true, Ordering::Relaxed);
    }

    fn menu_action_resume(&self) {
        self.ui_to_nes_sender.send(UiEvent::Resume).unwrap();
        self.paused.store(false, Ordering::Relaxed);
    }

    fn paint(&self, ctrl: &ExternCanvas, data: &EventData) {
        let paint = data.on_paint();
        let ps = paint.begin_paint();

        let hdc = ps.hdc;
        let rc = &ps.rcPaint;

        let data: *const u8 = self.image.lock().unwrap().as_ptr();

        // All/Most functions from the winapi are unsafe, so ya
        unsafe {
            // clear only if needed, clearing every time produce flickering effect
            if self.need_clear_rect.get() {
                let brush: *mut _ = &mut CreateSolidBrush(RGB(0, 0, 0));
                FillRect(hdc, rc, brush as _);
                self.need_clear_rect.set(false);
            }

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
        if let Ok(event) = self.nes_to_ui_receiver.try_recv() {
            match event {
                BackendEvent::PresentStates(states) => {
                    let save_menu_ctrl: ControlHandle = (&self.file_menu_save_state_menu).into();
                    let load_menu_ctrl: ControlHandle = (&self.file_menu_load_state_menu).into();

                    if let Some((_s_hmenu_parent, save_hmenu)) = save_menu_ctrl.hmenu() {
                        if let Some((_l_hmenu_parent, load_hmenu)) = load_menu_ctrl.hmenu() {
                            use winapi::um::winuser::{GetMenuItemID, ModifyMenuA};
                            use winapi::um::winuser::{MF_BYPOSITION, MF_STRING};

                            for i in states {
                                let i = i - 1;

                                let label = format!("&{} saved\0", i + 1);

                                // FIXME: add SAFETY argument (windows API)
                                unsafe {
                                    let id = GetMenuItemID(save_hmenu, i as i32);

                                    ModifyMenuA(
                                        save_hmenu,
                                        i as u32,
                                        MF_STRING | MF_BYPOSITION,
                                        id as usize,
                                        label.as_ptr() as _,
                                    );

                                    let id = GetMenuItemID(load_hmenu, i as i32);

                                    ModifyMenuA(
                                        load_hmenu,
                                        i as u32,
                                        MF_STRING | MF_BYPOSITION,
                                        id as usize,
                                        label.as_ptr() as _,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        self.canvas.invalidate();
    }
}

pub struct NwgProvider {}

impl NwgProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl UiProvider for NwgProvider {
    fn get_tv_color_converter() -> fn(&NESColor) -> [u8; 4] {
        |color| [color.b, color.g, color.r, 0xFF]
    }

    fn run_ui_loop(
        &mut self,
        ui_to_nes_sender: Sender<UiEvent>,
        nes_to_ui_receiver: Receiver<BackendEvent>,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<StandardNESControllerState>>,
    ) {
        nwg::init().expect("Failed to init Native Windows GUI");
        nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

        let _app = ProviderApp::build_ui(ProviderApp::initial_state(
            ui_to_nes_sender,
            nes_to_ui_receiver,
            image,
            ctrl_state,
        ))
        .expect("Failed to build UI");

        nwg::dispatch_thread_events();
    }
}
