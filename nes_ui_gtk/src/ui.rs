use plastic_core::{
    nes_controller::{StandardNESControllerState, StandardNESKey},
    nes_display::{Color as NESColor, TV_HEIGHT, TV_WIDTH},
    BackendEvent, UiEvent, UiProvider,
};
use std::sync::{
    atomic::AtomicBool,
    atomic::Ordering,
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

use gdk::{keys::constants as key_vals, keyval_to_upper, DragAction, ModifierType};
use gdk_pixbuf::prelude::*;
use gdk_pixbuf::PixbufLoader;
use gio::prelude::*;
use glib::source::timeout_add_local;
use gtk::prelude::*;
use gtk::{
    Application, Builder, DestDefaults, DrawingArea, FileChooserAction, FileChooserDialog,
    FileFilter, Inhibit, Menu, MenuItem, ResponseType, TargetEntry, TargetFlags, Window,
};

const NUMBER_OF_STATES: u8 = 10;

pub struct GtkProvider {
    paused: Arc<AtomicBool>,
}

impl GtkProvider {
    pub fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl UiProvider for GtkProvider {
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
        gtk::init().unwrap();

        let app = Application::new(
            Some("amjad50.plastic.nes_gtk"),
            gio::ApplicationFlags::NON_UNIQUE,
        )
        .expect("Application could not be initialized");

        let ui_glade_string = include_str!("../ui.glade");
        let builder = Builder::from_string(ui_glade_string);

        let window = builder.get_object::<Window>("top_level_window").unwrap();
        let drawing_area = builder.get_object::<DrawingArea>("canvas").unwrap();
        let menu_action_open = builder.get_object::<MenuItem>("menu_action_open").unwrap();
        let save_state_menu_list = builder.get_object::<Menu>("save_state_menu").unwrap();
        let load_state_menu_list = builder.get_object::<Menu>("load_state_menu").unwrap();
        let menu_action_quit = builder.get_object::<MenuItem>("menu_action_quit").unwrap();
        let menu_action_reset = builder.get_object::<MenuItem>("menu_action_reset").unwrap();
        let menu_action_pause = builder.get_object::<MenuItem>("menu_action_pause").unwrap();
        let menu_action_resume = builder
            .get_object::<MenuItem>("menu_action_resume")
            .unwrap();

        // setup window icon
        let icon_png_raw_data = include_bytes!("../../images/icon.png");
        let icon_loader = PixbufLoader::with_type("png").unwrap();
        icon_loader.write(icon_png_raw_data).unwrap();
        icon_loader.close().unwrap();

        if let Some(icon) = icon_loader.get_pixbuf() {
            window.set_icon(Some(&icon));
        }

        for i in 1..=NUMBER_OF_STATES {
            let save_action = MenuItem::with_mnemonic(&format!("_{} <empty>", i));
            let load_action = MenuItem::with_mnemonic(&format!("_{} <empty>", i));

            // setup handlers
            {
                let ui_to_nes_sender = ui_to_nes_sender.clone();
                save_action.connect_activate(move |_| {
                    ui_to_nes_sender.send(UiEvent::SaveState(i)).unwrap();
                });
            }
            {
                let ui_to_nes_sender = ui_to_nes_sender.clone();
                load_action.connect_activate(move |_| {
                    ui_to_nes_sender.send(UiEvent::LoadState(i)).unwrap();
                });
            }

            // add the actions to the menus
            save_state_menu_list.append(&save_action);
            load_state_menu_list.append(&load_action);
        }

        window.show_all();

        drawing_area.connect_draw(move |area, cr| {
            let data = image.lock().unwrap().to_vec();
            let src = cairo::ImageSurface::create_for_data(
                data,
                cairo::Format::Rgb24,
                TV_WIDTH as i32,
                TV_HEIGHT as i32,
                cairo::Format::Rgb24
                    .stride_for_width(TV_WIDTH as u32)
                    .unwrap(),
            )
            .unwrap();
            let pattern = cairo::SurfacePattern::create(&src);
            pattern.set_filter(cairo::Filter::Nearest);

            let area_width = area.get_allocated_width() as f64;
            let area_height = area.get_allocated_height() as f64;

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

            cr.translate(left, top);

            cr.scale(scale_smallest, scale_smallest);

            cr.set_source(&pattern);
            cr.paint();

            Inhibit(false)
        });

        window.set_size_request((TV_WIDTH * 3) as i32, (TV_HEIGHT * 3) as i32);

        {
            let ctrl_state = ctrl_state.clone();
            let ui_to_nes_sender = ui_to_nes_sender.clone();
            let paused = self.paused.clone();
            window.connect_key_press_event(move |_, event| {
                let mut ctrl = ctrl_state.lock().unwrap();

                // should be fixed with https://github.com/gtk-rs/gdk/pull/358
                let mut val = event.get_keyval();
                *val = keyval_to_upper(*val);
                match val {
                    key_vals::J => ctrl.press(StandardNESKey::B),
                    key_vals::K => ctrl.press(StandardNESKey::A),
                    key_vals::U => ctrl.press(StandardNESKey::Select),
                    key_vals::I => ctrl.press(StandardNESKey::Start),
                    key_vals::W => ctrl.press(StandardNESKey::Up),
                    key_vals::S => ctrl.press(StandardNESKey::Down),
                    key_vals::A => ctrl.press(StandardNESKey::Left),
                    key_vals::D => ctrl.press(StandardNESKey::Right),
                    key_vals::R if event.get_state().intersects(ModifierType::CONTROL_MASK) => {
                        ui_to_nes_sender.send(UiEvent::Reset).unwrap()
                    }
                    key_vals::Escape => {
                        if paused.load(Ordering::Relaxed) {
                            ui_to_nes_sender.send(UiEvent::Resume).unwrap();
                            paused.store(false, Ordering::Relaxed);
                        } else {
                            ui_to_nes_sender.send(UiEvent::Pause).unwrap();
                            paused.store(true, Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }

                Inhibit(false)
            });
        }

        {
            window.connect_key_release_event(move |_, event| {
                let mut ctrl = ctrl_state.lock().unwrap();

                // should be fixed with https://github.com/gtk-rs/gdk/pull/358
                let mut val = event.get_keyval();
                *val = keyval_to_upper(*val);
                match val {
                    key_vals::J => ctrl.release(StandardNESKey::B),
                    key_vals::K => ctrl.release(StandardNESKey::A),
                    key_vals::U => ctrl.release(StandardNESKey::Select),
                    key_vals::I => ctrl.release(StandardNESKey::Start),
                    key_vals::W => ctrl.release(StandardNESKey::Up),
                    key_vals::S => ctrl.release(StandardNESKey::Down),
                    key_vals::A => ctrl.release(StandardNESKey::Left),
                    key_vals::D => ctrl.release(StandardNESKey::Right),
                    _ => {}
                }

                Inhibit(false)
            });
        }

        // Support for dragging a new file into the emulator
        const DRAG_ID: u32 = 100;
        window.drag_dest_set(
            DestDefaults::ALL,
            &[TargetEntry::new(
                "text/plain",
                TargetFlags::OTHER_APP,
                DRAG_ID,
            )],
            DragAction::COPY,
        );

        {
            let ui_to_nes_sender = ui_to_nes_sender.clone();
            window.connect_drag_data_received(move |_, _, _x, _y, data, info, _| {
                if info == DRAG_ID {
                    if let Some(text) = data.get_text() {
                        let text = text.trim_start_matches("file://");

                        // we don't want to panic and exit, just ignore if corrupted
                        if text.ends_with(".nes") {
                            ui_to_nes_sender
                                .send(UiEvent::LoadRom(text.to_owned()))
                                .unwrap();
                        }
                    }
                }
            });
        }

        {
            let ui_to_nes_sender = ui_to_nes_sender.clone();
            menu_action_reset.connect_activate(move |_| {
                ui_to_nes_sender.send(UiEvent::Reset).unwrap();
            });
        }

        {
            let ui_to_nes_sender = ui_to_nes_sender.clone();
            let paused = self.paused.clone();
            menu_action_pause.connect_activate(move |_| {
                ui_to_nes_sender.send(UiEvent::Pause).unwrap();
                paused.store(true, Ordering::Relaxed);
            });
        }

        {
            let ui_to_nes_sender = ui_to_nes_sender.clone();
            let paused = self.paused.clone();
            menu_action_resume.connect_activate(move |_| {
                ui_to_nes_sender.send(UiEvent::Resume).unwrap();
                paused.store(false, Ordering::Relaxed);
            });
        }

        {
            let app = app.clone();
            menu_action_quit.connect_activate(move |_| app.quit());
        }

        {
            let ui_to_nes_sender = ui_to_nes_sender.clone();
            menu_action_open.connect_activate(move |_| {
                let dialog = FileChooserDialog::with_buttons::<Window>(
                    Some("Select NES ROM"),
                    None,
                    FileChooserAction::Open,
                    &[
                        ("_Cancel", ResponseType::Cancel),
                        ("_Open", ResponseType::Accept),
                    ],
                );

                let filter = FileFilter::new();
                filter.add_mime_type("application/x-nes-rom");
                dialog.set_filter(&filter);

                let result = dialog.run();
                if result == ResponseType::Accept {
                    if let Some(file) = dialog.get_filename() {
                        ui_to_nes_sender
                            .send(UiEvent::LoadRom(file.to_string_lossy().to_string()))
                            .unwrap();
                    }
                }
                dialog.close();
            });
        }

        app.connect_activate(move |app| {
            app.add_window(&window);
        });

        timeout_add_local(1000 / 120, move || {
            if let Ok(event) = nes_to_ui_receiver.try_recv() {
                match event {
                    BackendEvent::PresentStates(states) => {
                        let mut states_labels = [false; NUMBER_OF_STATES as usize];

                        for i in states.iter() {
                            if let Some(ptr) = states_labels.get_mut(*i as usize - 1) {
                                *ptr = true;
                            }
                        }

                        for (i, &label) in states_labels.iter().enumerate() {
                            let label =
                                format!("_{} {}", i + 1, if label { "saved" } else { "<empty>" });

                            if let Some(item) = save_state_menu_list.get_children().get(i as usize)
                            {
                                if let Some(item) = item.downcast_ref::<MenuItem>() {
                                    item.set_label(&label);
                                }
                            }
                            if let Some(item) = load_state_menu_list.get_children().get(i as usize)
                            {
                                if let Some(item) = item.downcast_ref::<MenuItem>() {
                                    item.set_label(&label);
                                }
                            }
                        }
                    }
                }
            }

            drawing_area.queue_draw_area(
                0,
                0,
                drawing_area.get_allocated_width(),
                drawing_area.get_allocated_height(),
            );
            glib::Continue(true)
        });

        app.run(&[]);
        ui_to_nes_sender.send(UiEvent::Exit).unwrap();
    }
}
