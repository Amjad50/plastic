use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use directories::ProjectDirs;
use dynwave::AudioPlayer;
use egui_winit::winit::platform::x11::EventLoopBuilderExtX11 as _;
use plastic_core::{
    nes::NES,
    nes_audio::SAMPLE_RATE,
    nes_controller::StandardNESKey,
    nes_display::{TV_HEIGHT, TV_WIDTH},
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

struct MovingAverage {
    values: [f64; 100],
    current_index: usize,
    sum: f64,
}

impl MovingAverage {
    fn new() -> Self {
        Self {
            values: [0.0; 100],
            current_index: 0,
            sum: 0.0,
        }
    }

    fn add(&mut self, value: f64) {
        self.sum -= self.values[self.current_index];
        self.sum += value;
        self.values[self.current_index] = value;
        self.current_index = (self.current_index + 1) % self.values.len();
    }

    fn average(&self) -> f64 {
        self.sum / self.values.len() as f64
    }
}

/// Moving average fps counter
struct Fps {
    moving_average: MovingAverage,
    last_frame: Instant,
    target_fps: f64,
}

impl Fps {
    fn new(target_fps: f64) -> Self {
        Self {
            moving_average: MovingAverage::new(),
            last_frame: Instant::now(),
            target_fps,
        }
    }

    // check if we should start a new frame
    // return true if we should start a new frame
    // return false if we should skip this frame
    fn start_frame(&mut self) -> bool {
        let duration_per_frame = Duration::from_secs_f64(1.0 / self.target_fps);
        let elapsed = self.last_frame.elapsed();
        if elapsed < duration_per_frame {
            return false;
        }

        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        self.moving_average.add(delta);
        true
    }

    fn fps(&self) -> f64 {
        1.0 / self.moving_average.average()
    }

    /// Schedule the update so that the frame rate is capped at the target fps
    fn schedule_update(&mut self, ctx: &egui::Context) {
        let duration_per_frame = Duration::from_secs_f64(1.0 / self.target_fps);

        let elapsed = self.last_frame.elapsed();

        if elapsed >= duration_per_frame {
            ctx.request_repaint();
            return;
        }

        let remaining = duration_per_frame - elapsed;
        ctx.request_repaint_after(remaining);
    }
}

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
        if self.nes.is_empty() {
            return;
        }

        let base_saved_states_dir = base_save_state_folder().unwrap();
        let filename = self.nes.save_state_file_name(slot).unwrap();
        let path = base_saved_states_dir.join(&filename);

        let file = fs::File::create(&path).unwrap();

        self.nes.save_state(&file).unwrap();
    }

    fn load_state(&mut self, slot: u8) {
        if self.nes.is_empty() {
            return;
        }

        let base_saved_states_dir = base_save_state_folder().unwrap();
        let filename = self.nes.save_state_file_name(slot).unwrap();
        let path = base_saved_states_dir.join(&filename);

        let file = fs::File::open(&path).unwrap();

        self.nes.load_state(&file).unwrap();
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
                    .controller()
                    .set_state(StandardNESKey::B, i.key_down(egui::Key::J));
                self.nes
                    .controller()
                    .set_state(StandardNESKey::A, i.key_down(egui::Key::K));
                self.nes
                    .controller()
                    .set_state(StandardNESKey::Select, i.key_down(egui::Key::U));
                self.nes
                    .controller()
                    .set_state(StandardNESKey::Start, i.key_down(egui::Key::I));
                self.nes
                    .controller()
                    .set_state(StandardNESKey::Up, i.key_down(egui::Key::W));
                self.nes
                    .controller()
                    .set_state(StandardNESKey::Down, i.key_down(egui::Key::S));
                self.nes
                    .controller()
                    .set_state(StandardNESKey::Left, i.key_down(egui::Key::A));
                self.nes
                    .controller()
                    .set_state(StandardNESKey::Right, i.key_down(egui::Key::D));
            }
        });
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = format!(
            "Plastic ({:.0} FPS) {}",
            self.fps.fps(),
            if self.paused { "- Paused" } else { "" }
        );

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("NES ROM", &["nes"])
                        .pick_file()
                    {
                        self.nes = NES::new(file).unwrap();
                    }
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
                        .clamp_to_range(true),
                );
                self.fps.target_fps = TARGET_FPS * speed;
            });
        });
    }

    /// Process the audio buffer to make it stereo
    /// Also add or remove samples to match the current FPS difference from TARGET_FPS
    fn process_audio(&self, audio_buffer: &[f32]) -> Vec<f32> {
        let fps_ratio = TARGET_FPS / self.fps.target_fps;
        let target_len = (audio_buffer.len() as f64 * fps_ratio).ceil() as usize;
        let mut adjusted_buffer = Vec::with_capacity(target_len * 2);

        for i in 0..target_len {
            let src_index_f = i as f64 / fps_ratio;
            let src_index = src_index_f.floor() as usize;
            let next_index = std::cmp::min(src_index + 1, audio_buffer.len() - 1);
            let fraction = src_index_f.fract() as f32;

            let sample = if src_index < audio_buffer.len() {
                let current_sample = audio_buffer[src_index];
                let next_sample = audio_buffer[next_index];
                current_sample * (1.0 - fraction) + next_sample * fraction
            } else {
                *audio_buffer.last().unwrap_or(&0.0)
            };
            // Add the sample twice for left and right channels
            adjusted_buffer.push(sample);
            adjusted_buffer.push(sample);
        }

        adjusted_buffer
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
                self.audio_player.queue(&self.process_audio(&audio_buffer));
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

        self.fps.schedule_update(ctx);
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
            event_loop_builder: Some(Box::new(|builder| {
                builder.with_x11();
            })),
            window_builder: Some(Box::new(|builder| {
                builder.with_drag_and_drop(true).with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!("../../images/icon.png"))
                        .unwrap(),
                )
            })),
            vsync: false, // unlock FPS
            ..Default::default()
        },
        Box::new(|c| Ok(Box::new(App::new(&c.egui_ctx, nes)))),
    )
}
