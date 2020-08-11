use nes_ui_base::{
    nes::{TV_HEIGHT, TV_WIDTH},
    nes_controller::{StandardNESControllerState, StandardNESKey},
    nes_display::Color as NESColor,
    UiProvider,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gdk::enums::key;
use gio::prelude::*;
use gtk::prelude::*;

pub struct GtkProvider {}

impl UiProvider for GtkProvider {
    fn get_tv_color_converter() -> fn(&NESColor) -> [u8; 4] {
        |color| [color.b, color.g, color.r, 0xFF]
    }

    fn run_ui_loop(
        &mut self,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<StandardNESControllerState>>,
    ) {
        let ctrl_state1 = ctrl_state.clone();
        let ctrl_state2 = ctrl_state.clone();

        let app = gtk::Application::new(
            Some("amjad50.plastic.nes_gtk"),
            gio::ApplicationFlags::FLAGS_NONE,
        )
        .expect("Application could not be initialized");

        let window = Rc::new(RefCell::new(gtk::Window::new(gtk::WindowType::Toplevel)));

        window.borrow_mut().set_title("Plastic");

        let window_redraw = window.clone();
        let drawing_area = gtk::DrawingArea::new();

        drawing_area.connect_draw(move |area, cr| {
            let data = image.lock().unwrap().to_vec();
            let src = cairo::ImageSurface::create_for_data(
                data,
                cairo::Format::Rgb24,
                TV_WIDTH as i32,
                TV_HEIGHT as i32,
                cairo::Format::Rgb24.stride_for_width(TV_WIDTH).unwrap(),
            )
            .unwrap();
            let pattern = cairo::SurfacePattern::create(&src);
            pattern.set_filter(cairo::Filter::Nearest);

            let scale_width = area.get_allocated_width() as f64 / TV_WIDTH as f64;
            let scale_height = area.get_allocated_height() as f64 / TV_HEIGHT as f64;

            cr.scale(scale_width, scale_height);

            cr.set_source(&pattern);
            cr.paint();

            gtk::Inhibit(false)
        });

        window
            .borrow_mut()
            .set_size_request((TV_WIDTH * 3) as i32, (TV_HEIGHT * 3) as i32);

        window.borrow_mut().add(&drawing_area);

        let image = gtk::Image::new();

        window.borrow_mut().add(&image);

        window.borrow().show_all();

        window
            .borrow_mut()
            .connect_key_press_event(move |_, event| {
                let mut ctrl = ctrl_state1.lock().unwrap();
                match gdk::keyval_to_upper(event.get_keyval()) {
                    key::J => ctrl.press(StandardNESKey::B),
                    key::K => ctrl.press(StandardNESKey::A),
                    key::U => ctrl.press(StandardNESKey::Select),
                    key::I => ctrl.press(StandardNESKey::Start),
                    key::W => ctrl.press(StandardNESKey::Up),
                    key::S => ctrl.press(StandardNESKey::Down),
                    key::A => ctrl.press(StandardNESKey::Left),
                    key::D => ctrl.press(StandardNESKey::Right),
                    _ => {}
                }

                Inhibit(false)
            });

        window
            .borrow_mut()
            .connect_key_release_event(move |_, event| {
                let mut ctrl = ctrl_state2.lock().unwrap();
                match gdk::keyval_to_upper(event.get_keyval()) {
                    key::J => ctrl.release(StandardNESKey::B),
                    key::K => ctrl.release(StandardNESKey::A),
                    key::U => ctrl.release(StandardNESKey::Select),
                    key::I => ctrl.release(StandardNESKey::Start),
                    key::W => ctrl.release(StandardNESKey::Up),
                    key::S => ctrl.release(StandardNESKey::Down),
                    key::A => ctrl.release(StandardNESKey::Left),
                    key::D => ctrl.release(StandardNESKey::Right),
                    _ => {}
                }

                Inhibit(false)
            });

        app.connect_activate(move |app| {
            app.add_window(&*window.borrow());
        });

        timeout_add(1000 / 120, move || {
            let window = window_redraw.borrow();
            window.queue_draw_area(
                0,
                0,
                window.get_allocated_width(),
                window.get_allocated_height(),
            );
            glib::Continue(true)
        });

        app.run(&[]);
    }
}
