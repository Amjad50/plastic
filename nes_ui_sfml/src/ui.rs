use nes_ui_base::{
    nes_controller::{StandardNESControllerState, StandardNESKey},
    nes_display::{Color as NESColor, TV_HEIGHT, TV_WIDTH},
    BackendEvent, UiEvent, UiProvider,
};

use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

use sfml::{
    graphics::{Color, FloatRect, Image, RenderTarget, RenderWindow, Sprite, Texture, View},
    system::{SfBox, Vector2f},
    window::{joystick::Axis, Event, Key, Style},
};

pub struct SfmlProvider {}

impl SfmlProvider {
    /// calculate a new view based on the window size
    fn get_view(
        window_width: u32,
        window_height: u32,
        target_width: u32,
        target_height: u32,
    ) -> SfBox<View> {
        let mut viewport = FloatRect::new(0., 0., 1., 1.);

        let screen_width = window_width as f32 / target_width as f32;
        let screen_height = window_height as f32 / target_height as f32;

        if screen_width > screen_height {
            viewport.width = screen_height / screen_width;
            viewport.left = (1. - viewport.width) / 2.;
        } else if screen_height > screen_width {
            viewport.height = screen_width / screen_height;
            viewport.top = (1. - viewport.height) / 2.;
        }

        let mut view = View::new(
            Vector2f::new((TV_WIDTH / 2) as f32, (TV_HEIGHT / 2) as f32),
            Vector2f::new((TV_WIDTH) as f32, (TV_HEIGHT) as f32),
        );

        view.set_viewport(&viewport);

        view
    }
}
impl UiProvider for SfmlProvider {
    fn run_ui_loop(
        &mut self,
        ui_to_nes_sender: Sender<UiEvent>,
        _nes_to_ui_receiver: Receiver<BackendEvent>,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<StandardNESControllerState>>,
    ) {
        let mut window = RenderWindow::new(
            (TV_WIDTH as u32 * 3, TV_HEIGHT as u32 * 3),
            "Plastic",
            Style::CLOSE | Style::RESIZE,
            &Default::default(),
        );
        window.set_vertical_sync_enabled(true);
        window.set_framerate_limit(60);

        // to scale the view into the window
        // this view is in the size of the NES TV
        // but we can scale the window and all the pixels will be scaled
        // accordingly
        window.set_view(&Self::get_view(
            window.size().x,
            window.size().y,
            TV_WIDTH as u32,
            TV_HEIGHT as u32,
        ));

        let mut texture = Texture::new(TV_WIDTH as u32, TV_HEIGHT as u32).expect("texture");

        'main: loop {
            if let Ok(mut ctrl) = ctrl_state.lock() {
                while let Some(event) = window.poll_event() {
                    match event {
                        Event::Closed => break 'main,
                        Event::Resized { width, height } => {
                            window.set_view(&Self::get_view(
                                width,
                                height,
                                TV_WIDTH as u32,
                                TV_HEIGHT as u32,
                            ));
                        }
                        Event::KeyPressed {
                            code: key,
                            ctrl: ctrl_key,
                            ..
                        } => match key {
                            Key::J => ctrl.press(StandardNESKey::B),
                            Key::K => ctrl.press(StandardNESKey::A),
                            Key::U => ctrl.press(StandardNESKey::Select),
                            Key::I => ctrl.press(StandardNESKey::Start),
                            Key::W => ctrl.press(StandardNESKey::Up),
                            Key::S => ctrl.press(StandardNESKey::Down),
                            Key::A => ctrl.press(StandardNESKey::Left),
                            Key::D => ctrl.press(StandardNESKey::Right),
                            Key::R if ctrl_key => ui_to_nes_sender.send(UiEvent::Reset).unwrap(),
                            _ => {}
                        },
                        Event::KeyReleased { code: key, .. } => match key {
                            Key::J => ctrl.release(StandardNESKey::B),
                            Key::K => ctrl.release(StandardNESKey::A),
                            Key::U => ctrl.release(StandardNESKey::Select),
                            Key::I => ctrl.release(StandardNESKey::Start),
                            Key::W => ctrl.release(StandardNESKey::Up),
                            Key::S => ctrl.release(StandardNESKey::Down),
                            Key::A => ctrl.release(StandardNESKey::Left),
                            Key::D => ctrl.release(StandardNESKey::Right),
                            _ => {}
                        },
                        Event::JoystickButtonPressed {
                            joystickid: 0,
                            button,
                        } => match button {
                            0 => ctrl.press(StandardNESKey::B),
                            1 => ctrl.press(StandardNESKey::A),
                            8 => ctrl.press(StandardNESKey::Select),
                            9 => ctrl.press(StandardNESKey::Start),
                            _ => {}
                        },
                        Event::JoystickButtonReleased {
                            joystickid: 0,
                            button,
                        } => match button {
                            0 => ctrl.release(StandardNESKey::B),
                            1 => ctrl.release(StandardNESKey::A),
                            8 => ctrl.release(StandardNESKey::Select),
                            9 => ctrl.release(StandardNESKey::Start),
                            _ => {}
                        },
                        Event::JoystickMoved {
                            joystickid: 0,
                            axis,
                            position,
                        } => match axis {
                            Axis::PovY => {
                                if position > 0. {
                                    ctrl.press(StandardNESKey::Down)
                                } else if position < 0. {
                                    ctrl.press(StandardNESKey::Up)
                                } else {
                                    ctrl.release(StandardNESKey::Down);
                                    ctrl.release(StandardNESKey::Up);
                                }
                            }
                            Axis::PovX => {
                                if position > 0. {
                                    ctrl.press(StandardNESKey::Right)
                                } else if position < 0. {
                                    ctrl.press(StandardNESKey::Left)
                                } else {
                                    ctrl.release(StandardNESKey::Right);
                                    ctrl.release(StandardNESKey::Left);
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }

            window.clear(Color::BLACK);

            {
                let pixels = &image.lock().unwrap();

                let image = Image::create_from_pixels(TV_WIDTH as u32, TV_HEIGHT as u32, pixels)
                    .expect("image");

                texture.update_from_image(&image, 0, 0);
            }

            window.draw(&Sprite::with_texture(&texture));

            window.display();
        }
    }

    fn get_tv_color_converter() -> fn(&NESColor) -> [u8; 4] {
        |color| [color.r, color.g, color.b, 0xFF]
    }
}
