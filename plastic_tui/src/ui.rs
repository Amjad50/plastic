use directories::ProjectDirs;
use dynwave::AudioPlayer;
use layout::Flex;
use plastic_core::{
    misc::{process_audio, Fps},
    nes_audio::SAMPLE_RATE,
    nes_display::{TV_HEIGHT, TV_WIDTH},
    NESKey, NES,
};
use ratatui::{
    prelude::*,
    style::Color,
    widgets::{
        block::{Position, Title},
        canvas::{Canvas, Painter, Shape},
        Block, Borders, Clear, Padding, Paragraph,
    },
};
use ratatui_explorer::{FileExplorer, Theme};
use std::{collections::HashMap, fs, path::PathBuf, thread};
use std::{io, time::Duration};
use symbols::Marker;
use tui_menu::{Menu, MenuEvent as tuiMenuEvent, MenuItem, MenuState};

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

fn base_save_state_folder() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("Amjad50", "Plastic", "Plastic") {
        let base_saved_states_dir = proj_dirs.data_local_dir().join("saved_states");
        // Linux:   /home/../.local/share/plastic/saved_states
        // Windows: C:\Users\..\AppData\Local\Plastic\Plastic\data\saved_states
        // macOS:   /Users/../Library/Application Support/Amjad50.Plastic.Plastic/saved_states

        fs::create_dir_all(&base_saved_states_dir).ok()?;

        Some(base_saved_states_dir)
    } else {
        None
    }
}

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

#[derive(Debug, Clone, Copy)]
enum MenuEvent {
    FileOpen,
    FileReset,
    FilePause,
    FileClose,
    FileExit,

    SaveState(u8),
    LoadState(u8),
}

pub struct Ui {
    pub nes: NES,

    paused: bool,
    error: Option<String>,
    file_explorer: FileExplorer,
    is_file_explorer_open: bool,

    menu: MenuState<MenuEvent>,
    audio_player: Option<AudioPlayer<f32>>,
    gilrs: Option<Gilrs>,
    active_gamepad: Option<gilrs::GamepadId>,

    /// For terminals without support for `Release` key event, we keep the button pressed for some
    /// time
    keyboard_event_counter: HashMap<NESKey, u32>,
}

impl Ui {
    pub fn new(nes: NES, has_audio: bool) -> Self {
        let theme = Theme::default()
            .with_block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::reset().fg(Color::White).bg(Color::Black)),
            )
            .with_dir_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .with_highlight_dir_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            )
            .add_default_title()
            .with_title_bottom(|_| "Select .nes file".into());

        Ui {
            nes,

            paused: false,
            error: None,
            file_explorer: FileExplorer::with_theme(theme).unwrap(),
            is_file_explorer_open: false,
            menu: MenuState::new(vec![]),
            audio_player: if has_audio {
                AudioPlayer::new(SAMPLE_RATE, dynwave::BufferSize::QuarterSecond).ok()
            } else {
                None
            },
            gilrs: Gilrs::new().ok(),
            keyboard_event_counter: HashMap::new(),
            active_gamepad: None,
        }
    }

    fn get_present_save_states(&self) -> Option<Vec<(u8, bool)>> {
        const MIN_STATE_SLOT: u8 = 0;
        const MAX_STATE_SLOT: u8 = 9;
        if self.nes.is_empty() {
            return None;
        }

        let base_saved_states_dir = base_save_state_folder()?;

        Some(
            (MIN_STATE_SLOT..=MAX_STATE_SLOT)
                .map(|i| {
                    let filename = self.nes.save_state_file_name(i).unwrap();

                    (i, base_saved_states_dir.join(&filename).exists())
                })
                .collect(),
        )
    }

    fn save_state(&mut self, slot: u8) {
        if let Some(path) = self.get_save_state_path(slot) {
            let file = fs::File::create(&path).unwrap();
            self.nes.save_state(&file).unwrap();
        }
    }

    fn load_state(&mut self, slot: u8) {
        if let Some(path) = self.get_save_state_path(slot) {
            let file = fs::File::open(&path).unwrap();
            self.nes.load_state(&file).unwrap();
        }
    }

    fn get_save_state_path(&self, slot: u8) -> Option<std::path::PathBuf> {
        if self.nes.is_empty() {
            return None;
        }

        let base_saved_states_dir = base_save_state_folder()?;
        let filename = self.nes.save_state_file_name(slot)?;

        Some(base_saved_states_dir.join(filename))
    }

    fn reset_menu(&mut self) {
        let mut save_state_items = Vec::with_capacity(10);
        let mut load_state_items = Vec::with_capacity(10);

        if let Some(slots) = self.get_present_save_states() {
            for slot in slots {
                save_state_items.push(MenuItem::item(
                    format!(
                        "Slot {} - {}",
                        slot.0,
                        if slot.1 { "Overwrite" } else { "Save" }
                    ),
                    MenuEvent::SaveState(slot.0),
                ));
                load_state_items.push(MenuItem::item(
                    format!("Slot {}{}", slot.0, if slot.1 { " - Present" } else { "" }),
                    MenuEvent::LoadState(slot.0),
                ));
            }
        }

        self.menu = MenuState::new(vec![
            MenuItem::group(
                "File",
                vec![
                    MenuItem::item("Open", MenuEvent::FileOpen),
                    MenuItem::item("Reset", MenuEvent::FileReset),
                    MenuItem::item(
                        if self.paused { "Resume" } else { "Pause" },
                        MenuEvent::FilePause,
                    ),
                    MenuItem::item("Close", MenuEvent::FileClose),
                    MenuItem::item("Exit", MenuEvent::FileExit),
                ],
            ),
            MenuItem::group("Save State", save_state_items),
            MenuItem::group("Load State", load_state_items),
        ]);
    }

    fn display<T: Backend>(&mut self, terminal: &mut Terminal<T>, fps: &Fps) {
        terminal
            .draw(move |f| {
                let [top, main] =
                    Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(f.area());

                let mut block = Block::default()
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
                if self.paused {
                    block = block.title(Title::from("[Paused]").alignment(Alignment::Center));
                }
                if let Some(error) = &self.error {
                    block = block
                        .title(
                            Title::from("Error:".red().bold())
                                .position(Position::Bottom)
                                .alignment(Alignment::Center),
                        )
                        .title(
                            Title::from(error.as_str().red().bold())
                                .position(Position::Bottom)
                                .alignment(Alignment::Center),
                        );
                }

                if self.nes.is_empty() {
                    let paragraph = Paragraph::new("No ROM loaded")
                        .block(block.padding(Padding::top(main.height / 2)))
                        .alignment(Alignment::Center);
                    f.render_widget(paragraph, main);
                } else {
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

                    f.render_widget(canvas, main);
                }

                if self.is_file_explorer_open {
                    // draw in a center of the screen
                    let horizontal =
                        Layout::horizontal([Constraint::Percentage(50)]).flex(Flex::Center);
                    let vertical =
                        Layout::vertical([Constraint::Percentage(70)]).flex(Flex::Center);
                    let [area] = vertical.areas(main);
                    let [file_exp_area] = horizontal.areas(area);
                    f.render_widget(Clear, file_exp_area);
                    f.render_widget(&self.file_explorer.widget(), file_exp_area);
                }

                f.render_stateful_widget(Menu::new(), top, &mut self.menu);
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
            if let Event::Key(input) = event {
                let modifiers = input.modifiers;
                let code = input.code;
                let is_press =
                    input.kind == KeyEventKind::Press || input.kind == KeyEventKind::Repeat;
                let possible_button = match code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => return true,
                    KeyCode::Char('C') | KeyCode::Char('c')
                        if modifiers.intersects(KeyModifiers::CONTROL) =>
                    {
                        return true
                    }
                    KeyCode::Char('R') | KeyCode::Char('r')
                        if modifiers.intersects(KeyModifiers::CONTROL) && is_press =>
                    {
                        self.nes.reset();
                        None
                    }
                    KeyCode::Char('J') | KeyCode::Char('j') => Some(NESKey::B),
                    KeyCode::Char('K') | KeyCode::Char('k') => Some(NESKey::A),
                    KeyCode::Char('U') | KeyCode::Char('u') => Some(NESKey::Select),
                    KeyCode::Char('I') | KeyCode::Char('i') => Some(NESKey::Start),
                    KeyCode::Char('W') | KeyCode::Char('w') => Some(NESKey::Up),
                    KeyCode::Char('S') | KeyCode::Char('s') => Some(NESKey::Down),
                    KeyCode::Char('A') | KeyCode::Char('a') => Some(NESKey::Left),
                    KeyCode::Char('D') | KeyCode::Char('d') => Some(NESKey::Right),
                    KeyCode::Char('P') | KeyCode::Char('p') if is_press => {
                        self.paused = !self.paused;
                        None
                    }
                    KeyCode::Enter if is_press => {
                        if self.is_file_explorer_open {
                            let file = self.file_explorer.current();
                            if !file.is_dir() {
                                if file.path().extension().map(|e| e == "nes").unwrap_or(false) {
                                    let new_nes = NES::new(file.path());
                                    match new_nes {
                                        Ok(nes) => {
                                            self.nes = nes;
                                            self.is_file_explorer_open = false;
                                        }
                                        Err(e) => {
                                            self.error = Some(format!("Opening NES: {}", e));
                                        }
                                    }

                                    self.error = None;
                                } else {
                                    self.error = Some("Invalid file".to_string());
                                }
                            }
                        } else {
                            self.menu.select();
                        }
                        None
                    }
                    KeyCode::Esc if is_press => {
                        self.is_file_explorer_open = false;
                        self.reset_menu();
                        None
                    }
                    KeyCode::Left if !self.is_file_explorer_open && is_press => {
                        self.menu.left();
                        None
                    }
                    KeyCode::Right if !self.is_file_explorer_open && is_press => {
                        self.menu.right();
                        None
                    }
                    KeyCode::Up if !self.is_file_explorer_open && is_press => {
                        self.menu.up();
                        None
                    }
                    KeyCode::Down if !self.is_file_explorer_open && is_press => {
                        self.menu.down();
                        None
                    }
                    _ => None,
                };
                if let Some(button) = possible_button {
                    if is_press {
                        self.nes.set_controller_state(button, true);
                        if !has_keyboard_enhancement {
                            // 20 frames
                            // TODO: very arbitrary, but it works on some of the games
                            // tested
                            self.keyboard_event_counter.insert(button, 20);
                        }
                    } else {
                        self.nes.set_controller_state(button, false);
                    }
                }
            }

            if self.is_file_explorer_open {
                self.file_explorer.handle(&event).unwrap();
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
                    self.nes.set_controller_state(*key, false);
                    false
                } else {
                    true
                }
            });
        }

        false
    }

    fn handle_menu(&mut self) -> bool {
        for e in self.menu.drain_events() {
            match e {
                tuiMenuEvent::Selected(event) => match event {
                    MenuEvent::FileOpen => {
                        self.is_file_explorer_open = true;
                    }
                    MenuEvent::FileReset => self.nes.reset(),
                    MenuEvent::FilePause => self.paused = !self.paused,
                    MenuEvent::FileClose => self.nes = NES::new_without_file(),
                    MenuEvent::FileExit => return true,
                    MenuEvent::SaveState(i) => self.save_state(i),
                    MenuEvent::LoadState(i) => self.load_state(i),
                },
            }
            self.reset_menu();
        }
        false
    }

    fn handle_gamepad(&mut self) {
        // set events in the cache and check if gamepad is still active
        if self.gilrs.is_none() {
            return;
        }
        

        while let Some(GilrsEvent { id, event, .. }) = self.gilrs.as_mut().unwrap().next_event() {
            self.active_gamepad = Some(id);
            if event == EventType::Disconnected {
                self.active_gamepad = None;
            }
        }

        if let Some(gamepad) = self.active_gamepad.map(|id| self.gilrs.as_mut().unwrap().gamepad(id)) {
            for (controller_button, nes_button) in &[
                (Button::South, NESKey::B),
                (Button::East, NESKey::A),
                (Button::Select, NESKey::Select),
                (Button::Start, NESKey::Start),
                (Button::DPadUp, NESKey::Up),
                (Button::DPadDown, NESKey::Down),
                (Button::DPadRight, NESKey::Right),
                (Button::DPadLeft, NESKey::Left),
            ] {
                if gamepad.is_pressed(*controller_button) {
                    self.nes.set_controller_state(*nes_button, true);
                } else {
                    self.nes.set_controller_state(*nes_button, false);
                }
            }
        }
    }

    pub fn run(&mut self) {
        self.reset_menu();

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

        loop {
            if let Some(ref mut player) = self.audio_player {
                if self.paused {
                    player.pause().unwrap();
                } else {
                    player.play().unwrap();
                }
            }

            fps.start_frame();
            if self.handle_keyboard(has_keyboard_enhancement) {
                break;
            }
            if self.handle_menu() {
                break;
            }
            self.handle_gamepad();

            if !self.paused {
                self.nes.clock_for_frame();
            }
            self.display(&mut terminal, &fps);

            // take the buffer in all cases, otherwise the audio will keep accumulating in memory
            let audio_buffer = self.nes.audio_buffer();
            if let Some(ref mut player) = self.audio_player {
                let audio_buffer = process_audio(&audio_buffer, 1.0);
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
