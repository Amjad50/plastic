use plastic_core::{
    nes_controller::{StandardNESControllerState, StandardNESKey},
    nes_display::{Color as NESColor, TV_HEIGHT, TV_WIDTH},
    BackendEvent, UiEvent, UiProvider,
};
use std::collections::HashSet;
use std::io;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use crate::event::{Event as tuiEvent, Events};

use gilrs::{Button, Event, EventType, Gilrs};

use crossterm::event::{KeyCode, KeyModifiers};
use tui::{
    backend::CrosstermBackend,
    style::Color,
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Shape},
        Block, Borders,
    },
    Terminal,
};

struct ImageView {
    image: Arc<Mutex<Vec<u8>>>,
}

impl Shape for ImageView {
    fn draw(&self, painter: &mut tui::widgets::canvas::Painter) {
        let data = self.image.lock().unwrap().to_vec();

        for x in 0..TV_WIDTH {
            for y in 0..TV_HEIGHT {
                let index = (TV_HEIGHT - y - 1) * TV_WIDTH + x;
                if let Some((x, y)) = painter.get_point(x as f64, y as f64) {
                    let pixel = data.get(index * 4..(index + 1) * 4).unwrap();
                    painter.paint(x, y, Color::Rgb(pixel[0], pixel[1], pixel[2]));
                }
            }
        }
    }
}

pub struct TuiProvider {}

impl UiProvider for TuiProvider {
    fn get_tv_color_converter() -> fn(&NESColor) -> [u8; 4] {
        |color| [color.r, color.g, color.b, 0xFF]
    }

    fn run_ui_loop(
        &mut self,
        ui_to_nes_sender: Sender<UiEvent>,
        _nes_to_ui_receiver: Receiver<BackendEvent>,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<StandardNESControllerState>>,
    ) {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut gilrs = Gilrs::new().unwrap();

        let mut active_gamepad = None;

        let keyboard_events = Events::new(Duration::from_millis(1000 / 30));

        // FIXME: find better way to handle input
        let mut not_pressed = HashSet::new();
        let mut pressed = HashSet::new();
        not_pressed.insert(StandardNESKey::A);
        not_pressed.insert(StandardNESKey::B);
        not_pressed.insert(StandardNESKey::Select);
        not_pressed.insert(StandardNESKey::Start);
        not_pressed.insert(StandardNESKey::Up);
        not_pressed.insert(StandardNESKey::Down);
        not_pressed.insert(StandardNESKey::Left);
        not_pressed.insert(StandardNESKey::Right);

        'outer: loop {
            let image = image.clone();
            terminal
                .draw(move |f| {
                    let canvas = Canvas::default()
                        .block(Block::default().borders(Borders::ALL).title("Plastic"))
                        .x_bounds([0., TV_WIDTH as f64])
                        .y_bounds([0., TV_HEIGHT as f64])
                        .marker(Marker::Dot)
                        .paint(|ctx| {
                            ctx.draw(&ImageView {
                                image: image.clone(),
                            });
                        });
                    f.render_widget(canvas, f.size());
                })
                .unwrap();

            if let Ok(mut ctrl) = ctrl_state.lock() {
                while let Ok(event) = keyboard_events.next() {
                    match event {
                        tuiEvent::Input(input) => {
                            let modifiers = input.modifiers;
                            let input = input.code;
                            let possible_button = match input {
                                KeyCode::Esc => break 'outer,
                                KeyCode::Char('R') | KeyCode::Char('r')
                                    if modifiers.intersects(KeyModifiers::CONTROL) =>
                                {
                                    ui_to_nes_sender.send(UiEvent::Reset).unwrap();
                                    None
                                }
                                KeyCode::Char('J') | KeyCode::Char('j') => Some(StandardNESKey::B),
                                KeyCode::Char('K') | KeyCode::Char('k') => Some(StandardNESKey::A),
                                KeyCode::Char('U') | KeyCode::Char('u') => {
                                    Some(StandardNESKey::Select)
                                }
                                KeyCode::Char('I') | KeyCode::Char('i') => {
                                    Some(StandardNESKey::Start)
                                }
                                KeyCode::Char('W') | KeyCode::Char('w') => Some(StandardNESKey::Up),
                                KeyCode::Char('S') | KeyCode::Char('s') => {
                                    Some(StandardNESKey::Down)
                                }
                                KeyCode::Char('A') | KeyCode::Char('a') => {
                                    Some(StandardNESKey::Left)
                                }
                                KeyCode::Char('D') | KeyCode::Char('d') => {
                                    Some(StandardNESKey::Right)
                                }
                                _ => None,
                            };
                            if let Some(button) = possible_button {
                                not_pressed.remove(&button);
                                pressed.insert(button);
                            }
                        }
                        tuiEvent::Tick => {
                            for button in &not_pressed {
                                ctrl.release(*button);
                            }
                            for button in &pressed {
                                ctrl.press(*button);
                            }

                            not_pressed.insert(StandardNESKey::A);
                            not_pressed.insert(StandardNESKey::B);
                            not_pressed.insert(StandardNESKey::Select);
                            not_pressed.insert(StandardNESKey::Start);
                            not_pressed.insert(StandardNESKey::Up);
                            not_pressed.insert(StandardNESKey::Down);
                            not_pressed.insert(StandardNESKey::Left);
                            not_pressed.insert(StandardNESKey::Right);

                            pressed.clear();
                        }
                    }
                }

                // set events in the cache and check if gamepad is still active
                while let Some(Event { id, event, .. }) = gilrs.next_event() {
                    active_gamepad = Some(id);
                    if event == EventType::Disconnected {
                        keyboard_events.set_stopped_state(false);
                        active_gamepad = None;
                    }
                }
                keyboard_events.set_stopped_state(active_gamepad.is_some());

                if let Some(gamepad) = active_gamepad.map(|id| gilrs.gamepad(id)) {
                    for (controller_button, nes_button) in &[
                        (Button::South, StandardNESKey::B),
                        (Button::East, StandardNESKey::A),
                        (Button::Select, StandardNESKey::Select),
                        (Button::Start, StandardNESKey::Start),
                        (Button::DPadUp, StandardNESKey::Up),
                        (Button::DPadDown, StandardNESKey::Down),
                        (Button::DPadRight, StandardNESKey::Right),
                        (Button::DPadLeft, StandardNESKey::Left),
                    ] {
                        if gamepad.is_pressed(*controller_button) {
                            ctrl.press(*nes_button);
                        } else {
                            ctrl.release(*nes_button);
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
