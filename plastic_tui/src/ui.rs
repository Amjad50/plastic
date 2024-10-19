use dynwave::AudioPlayer;
use plastic_core::{
    misc::{process_audio, Fps},
    nes::NES,
    nes_audio::SAMPLE_RATE,
    nes_controller::StandardNESKey,
    nes_display::{TV_HEIGHT, TV_WIDTH},
};
use ratatui::{
    prelude::*,
    style::Color,
    widgets::{
        block::Title,
        canvas::{Canvas, Painter, Shape},
        Block, Borders,
    },
};
use std::{collections::HashMap, thread};
use std::{io, time::Duration};
use symbols::Marker;

use gilrs::{Button, Event as GilrsEvent, EventType, Gilrs};

use crossterm::{
    cursor::{Hide, Show},
    event::{
        Event, KeyCode, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

struct ImageView<'a> {
    image: &'a [u8],
}

impl Shape for ImageView<'_> {
    fn draw(&self, painter: &mut Painter) {
        for x in 0..TV_WIDTH {
            for y in 0..TV_HEIGHT {
                let index = (TV_HEIGHT - y - 1) * TV_WIDTH + x;
                if let Some((x, y)) = painter.get_point(x as f64, y as f64) {
                    let r = self.image[index * 3];
                    let g = self.image[index * 3 + 1];
                    let b = self.image[index * 3 + 2];
                    painter.paint(x, y, Color::Rgb(r, g, b));
                }
            }
        }
    }
}

pub struct Ui {
    pub nes: NES,

    audio_player: Option<AudioPlayer<f32>>,
    gilrs: Gilrs,
    active_gamepad: Option<gilrs::GamepadId>,

    /// For terminals without support for `Release` key event, we keep the button pressed for some
    /// time
    keyboard_event_counter: HashMap<StandardNESKey, u32>,
}

impl Ui {
    pub fn new(nes: NES, has_audio: bool) -> Self {
        Ui {
            nes,

            audio_player: if has_audio {
                Some(AudioPlayer::new(SAMPLE_RATE, dynwave::BufferSize::QuarterSecond).unwrap())
            } else {
                None
            },
            gilrs: Gilrs::new().unwrap(),
            keyboard_event_counter: HashMap::new(),
            active_gamepad: None,
        }
    }

    fn display<T: Backend>(&mut self, terminal: &mut Terminal<T>, fps: &Fps) {
        terminal
            .draw(move |f| {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(Title::from("Plastic").alignment(Alignment::Center))
                    .title(
                        Title::from(format!("(FPS: {:.2})", fps.fps())).alignment(Alignment::Left),
                    )
                    .title(
                        Title::from(format!(
                            "(Terminal size: {}x{})",
                            f.area().width,
                            f.area().height
                        ))
                        .alignment(Alignment::Right),
                    )
                    .title_style(Style::default().bold().fg(Color::Yellow));
                let canvas = Canvas::default()
                    .block(block)
                    .x_bounds([0., TV_WIDTH as f64])
                    .y_bounds([0., TV_HEIGHT as f64])
                    .marker(Marker::HalfBlock)
                    .paint(|ctx| {
                        ctx.draw(&ImageView {
                            image: self.nes.pixel_buffer(),
                        });
                    });
                f.render_widget(canvas, f.area());
            })
            .unwrap();
    }

    fn handle_keyboard(&mut self, has_keyboard_enhancement: bool) -> bool {
        // read them in
        while let Ok(has_event) = crossterm::event::poll(Duration::from_millis(5)) {
            if !has_event {
                break;
            }
            let Ok(event) = crossterm::event::read() else {
                break;
            };
            match event {
                Event::Key(input) => {
                    let modifiers = input.modifiers;
                    let code = input.code;
                    let possible_button = match code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return true,

                        KeyCode::Char('C') | KeyCode::Char('c')
                            if modifiers.intersects(KeyModifiers::CONTROL) =>
                        {
                            return true
                        }
                        KeyCode::Char('R') | KeyCode::Char('r')
                            if modifiers.intersects(KeyModifiers::CONTROL) =>
                        {
                            self.nes.reset();
                            None
                        }
                        KeyCode::Char('J') | KeyCode::Char('j') => Some(StandardNESKey::B),
                        KeyCode::Char('K') | KeyCode::Char('k') => Some(StandardNESKey::A),
                        KeyCode::Char('U') | KeyCode::Char('u') => Some(StandardNESKey::Select),
                        KeyCode::Char('I') | KeyCode::Char('i') => Some(StandardNESKey::Start),
                        KeyCode::Char('W') | KeyCode::Char('w') => Some(StandardNESKey::Up),
                        KeyCode::Char('S') | KeyCode::Char('s') => Some(StandardNESKey::Down),
                        KeyCode::Char('A') | KeyCode::Char('a') => Some(StandardNESKey::Left),
                        KeyCode::Char('D') | KeyCode::Char('d') => Some(StandardNESKey::Right),
                        _ => None,
                    };
                    if let Some(button) = possible_button {
                        match input.kind {
                            KeyEventKind::Press | KeyEventKind::Repeat => {
                                self.nes.controller().set_state(button, true);
                                if !has_keyboard_enhancement {
                                    // 20 frames
                                    // TODO: very arbitrary, but it works on some of the games
                                    // tested
                                    self.keyboard_event_counter.insert(button, 20);
                                }
                            }
                            KeyEventKind::Release => {
                                self.nes.controller().set_state(button, false);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // decrement the counter for the keys that are being held
        if !has_keyboard_enhancement {
            self.keyboard_event_counter
                .iter_mut()
                .for_each(|(_, counter)| {
                    *counter = counter.saturating_sub(1);
                });

            self.keyboard_event_counter.retain(|key, counter| {
                if *counter == 0 {
                    self.nes.controller().set_state(*key, false);
                    false
                } else {
                    true
                }
            });
        }

        false
    }

    fn handle_gamepad(&mut self) {
        // set events in the cache and check if gamepad is still active
        while let Some(GilrsEvent { id, event, .. }) = self.gilrs.next_event() {
            self.active_gamepad = Some(id);
            if event == EventType::Disconnected {
                self.active_gamepad = None;
            }
        }

        if let Some(gamepad) = self.active_gamepad.map(|id| self.gilrs.gamepad(id)) {
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
                    self.nes.controller().set_state(*nes_button, true);
                } else {
                    self.nes.controller().set_state(*nes_button, false);
                }
            }
        }
    }

    pub fn run(&mut self) {
        let mut stdout = io::stdout();

        execute!(
            stdout,
            EnterAlternateScreen,
            Hide,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES),
        )
        .unwrap();
        enable_raw_mode().unwrap();

        let has_keyboard_enhancement =
            crossterm::terminal::supports_keyboard_enhancement().unwrap();

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut fps = Fps::new(61.0);

        if let Some(ref mut player) = self.audio_player {
            player.play().unwrap();
        }

        loop {
            fps.start_frame();
            if self.handle_keyboard(has_keyboard_enhancement) {
                break;
            }

            self.handle_gamepad();

            self.nes.clock_for_frame();
            self.display(&mut terminal, &fps);

            if let Some(ref mut player) = self.audio_player {
                let audio_buffer = process_audio(&self.nes.audio_buffer(), 1.0);
                player.queue(&audio_buffer);
            }

            if let Some(remaining) = fps.remaining() {
                thread::sleep(remaining);
            }
        }

        disable_raw_mode().unwrap();
        execute!(
            io::stdout(),
            Show,
            LeaveAlternateScreen,
            PopKeyboardEnhancementFlags,
        )
        .unwrap();
    }
}
