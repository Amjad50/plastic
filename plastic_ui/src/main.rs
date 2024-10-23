use std::{fs, path::PathBuf};

use directories::ProjectDirs;
use dynwave::AudioPlayer;
use plastic_core::{
    misc::{process_audio, Fps},
    nes_audio::SAMPLE_RATE,
    nes_display::{TV_HEIGHT, TV_WIDTH},
    NESKey, NES,
};

// 60 FPS gives audio glitches
const TARGET_FPS: f64 = 61.;

const MIN_STATE_SLOT: u8 = 0;
const MAX_STATE_SLOT: u8 = 9;

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

const OPEN_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::O);
const RESET_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::R);
const PAUSE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::P);
const CLOSE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Q);

struct App {
    fps: Fps,
    nes: NES,
    audio_player: AudioPlayer<f32>,
    image_texture: egui::TextureHandle,
    paused: bool,
}

impl App {
    pub fn new(ctx: &egui::Context, nes: NES) -> Self {
        Self {
            fps: Fps::new(TARGET_FPS),
            nes,
            audio_player: AudioPlayer::new(SAMPLE_RATE, dynwave::BufferSize::QuarterSecond)
                .unwrap(),
            paused: false,
            image_texture: ctx.load_texture(
                "nes-image",
                egui::ColorImage::from_rgb(
                    [TV_WIDTH, TV_HEIGHT],
                    vec![0; TV_WIDTH * TV_HEIGHT * 3].as_slice(),
                ),
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    minification: egui::TextureFilter::Nearest,
                    ..Default::default()
                },
            ),
        }
    }

    fn get_present_save_states(&self) -> Option<Vec<(u8, bool)>> {
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

    fn handle_input(&mut self, ctx: &egui::Context) {
        ctx.input_mut(|i| {
            if !i.raw.dropped_files.is_empty() {
                let file = i
                    .raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.as_ref())
                    .find(|f| f.extension().map(|e| e == "nes").unwrap_or(false));

                if let Some(file) = file {
                    self.nes = NES::new(file).unwrap();
                } else {
                    // convert to error alert
                    println!("[ERROR] Dropped file is not a NES ROM, must have .nes extension");
                }
            }
            if !i.focused {
                return;
            }

            if i.consume_shortcut(&OPEN_SHORTCUT) {
                self.open_file();
            }
            if i.consume_shortcut(&RESET_SHORTCUT) {
                self.nes.reset();
            }
            if i.consume_shortcut(&PAUSE_SHORTCUT) {
                self.paused = !self.paused;
                if !self.paused {
                    // clear the audio buffer
                    _ = self.nes.audio_buffer();
                }
            }
            if i.consume_shortcut(&CLOSE_SHORTCUT) {
                self.nes = NES::new_without_file();
            }

            if !self.nes.is_empty() {
                self.nes
                    .set_controller_state(NESKey::B, i.key_down(egui::Key::J));
                self.nes
                    .set_controller_state(NESKey::A, i.key_down(egui::Key::K));
                self.nes
                    .set_controller_state(NESKey::Select, i.key_down(egui::Key::U));
                self.nes
                    .set_controller_state(NESKey::Start, i.key_down(egui::Key::I));
                self.nes
                    .set_controller_state(NESKey::Up, i.key_down(egui::Key::W));
                self.nes
                    .set_controller_state(NESKey::Down, i.key_down(egui::Key::S));
                self.nes
                    .set_controller_state(NESKey::Left, i.key_down(egui::Key::A));
                self.nes
                    .set_controller_state(NESKey::Right, i.key_down(egui::Key::D));
            }
        });
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = format!(
            "Plastic {} {}",
            if self.nes.is_empty() || self.paused {
                "".to_owned()
            } else {
                format!("({:.0} FPS)", self.fps.fps())
            },
            if self.paused { "- Paused" } else { "" }
        );

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn open_file(&mut self) {
        if let Some(file) = rfd::FileDialog::new()
            .set_title("Open NES ROM")
            .add_filter("NES ROM", &["nes"])
            .pick_file()
        {
            self.nes = NES::new(file).unwrap();
        }
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .add(
                        egui::Button::new("Open")
                            .shortcut_text(ui.ctx().format_shortcut(&OPEN_SHORTCUT)),
                    )
                    .clicked()
                {
                    self.open_file();
                }
                if ui
                    .add(
                        egui::Button::new("Reset")
                            .shortcut_text(ui.ctx().format_shortcut(&RESET_SHORTCUT)),
                    )
                    .clicked()
                {
                    self.nes.reset();
                }
                if ui
                    .add(
                        egui::Button::new("Pause")
                            .selected(self.paused)
                            .shortcut_text(ui.ctx().format_shortcut(&PAUSE_SHORTCUT)),
                    )
                    .clicked()
                {
                    self.paused = !self.paused;
                    if !self.paused {
                        // clear the audio buffer
                        _ = self.nes.audio_buffer();
                    }
                }
                if ui
                    .add(
                        egui::Button::new("Close")
                            .shortcut_text(ui.ctx().format_shortcut(&CLOSE_SHORTCUT)),
                    )
                    .clicked()
                {
                    self.nes = NES::new_without_file();
                }
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Save State", |ui| {
                if let Some(slots) = self.get_present_save_states() {
                    for slot in slots {
                        if ui
                            .button(format!(
                                "Slot {} - {}",
                                slot.0,
                                if slot.1 { "Overwrite" } else { "Save" }
                            ))
                            .clicked()
                        {
                            self.save_state(slot.0);
                        }
                    }
                }
            });
            ui.menu_button("Load State", |ui| {
                if let Some(slots) = self.get_present_save_states() {
                    for slot in slots {
                        if ui
                            .add_enabled(slot.1, egui::Button::new(format!("Slot {}", slot.0)))
                            .clicked()
                            && slot.1
                        {
                            self.load_state(slot.0);
                        }
                    }
                }
            });
            ui.menu_button("Speed", |ui| {
                let mut speed = self.fps.target_fps / TARGET_FPS;
                ui.add(
                    egui::Slider::new(&mut speed, 0.1..=10.0)
                        .text("Emulation Speed")
                        .clamping(egui::SliderClamping::Always),
                );
                self.fps.target_fps = TARGET_FPS * speed;
            });
        });
    }

    /// Schedule the update so that the frame rate is capped at the target fps
    fn schedule_update(&mut self, ctx: &egui::Context) {
        if let Some(remaining) = self.fps.remaining() {
            ctx.request_repaint_after(remaining);
        } else {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_title(ctx);
        self.handle_input(ctx);

        if !self.paused && !self.nes.is_empty() {
            if self.fps.start_frame() {
                self.nes.clock_for_frame();
                let audio_buffer = self.nes.audio_buffer();
                self.audio_player.queue(&process_audio(
                    &audio_buffer,
                    (TARGET_FPS / self.fps.target_fps) as f32,
                ));
            }
            self.audio_player.play().unwrap();
        } else {
            self.audio_player.pause().unwrap();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_menu(ui);
            ui.centered_and_justified(|ui| {
                if !self.nes.is_empty() {
                    {
                        self.image_texture.set(
                            egui::ColorImage::from_rgb(
                                [TV_WIDTH, TV_HEIGHT],
                                self.nes.pixel_buffer(),
                            ),
                            egui::TextureOptions {
                                magnification: egui::TextureFilter::Nearest,
                                minification: egui::TextureFilter::Nearest,
                                ..Default::default()
                            },
                        );
                    }

                    let rect = ui.available_rect_before_wrap();

                    // image
                    ui.add(
                        egui::Image::from_texture(&self.image_texture)
                            .maintain_aspect_ratio(true)
                            .shrink_to_fit(),
                    );

                    // the pause indicator
                    if self.paused {
                        let center = rect.center();
                        let offset = 40.0;
                        let right_rect = egui::Rect::from_min_max(
                            center + egui::vec2(offset, -offset * 2.),
                            center + egui::vec2(offset + 40.0, offset * 2.),
                        );
                        let left_rect = egui::Rect::from_min_max(
                            center + egui::vec2(-offset - 40.0, -offset * 2.),
                            center + egui::vec2(-offset, offset * 2.),
                        );

                        ui.painter().rect_filled(
                            right_rect,
                            3.0,
                            egui::Color32::from_black_alpha(200),
                        );
                        ui.painter().rect_filled(
                            left_rect,
                            3.0,
                            egui::Color32::from_black_alpha(200),
                        );
                    }
                } else {
                    ui.label("No game loaded");
                }
            });
        });

        self.schedule_update(ctx);
    }
}

pub fn main() -> Result<(), eframe::Error> {
    let file = std::env::args().nth(1);
    let nes = match file {
        Some(file) => NES::new(&file).unwrap(),
        None => NES::new_without_file(),
    };

    eframe::run_native(
        "Plastic",
        eframe::NativeOptions {
            window_builder: Some(Box::new(|builder| {
                builder.with_drag_and_drop(true).with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!("../images/icon.png"))
                        .unwrap(),
                )
            })),
            vsync: false, // unlock FPS
            ..Default::default()
        },
        Box::new(|c| Ok(Box::new(App::new(&c.egui_ctx, nes)))),
    )
}
