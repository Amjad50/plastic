use dynwave::AudioPlayer;
use egui_winit::winit::platform::x11::EventLoopBuilderExtX11 as _;
use plastic_core::{
    nes::NES,
    nes_audio::SAMPLE_RATE,
    nes_controller::StandardNESKey,
    nes_display::{TV_HEIGHT, TV_WIDTH},
};

struct App {
    nes: NES,
    audio_player: AudioPlayer<f32>,
    image_texture: egui::TextureHandle,
    paused: bool,
    last_frame_time: std::time::Instant,
}

impl App {
    pub fn new(ctx: &egui::Context, nes: NES) -> Self {
        Self {
            nes,
            audio_player: AudioPlayer::new(SAMPLE_RATE, dynwave::BufferSize::QuarterSecond)
                .unwrap(),
            paused: false,
            last_frame_time: std::time::Instant::now(),
            image_texture: ctx.load_texture(
                "nes-image",
                egui::ColorImage::from_rgba_unmultiplied(
                    [TV_WIDTH, TV_HEIGHT],
                    vec![0; TV_WIDTH * TV_HEIGHT * 4].as_slice(),
                ),
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    minification: egui::TextureFilter::Nearest,
                    ..Default::default()
                },
            ),
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let file = i
                    .raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.as_ref())
                    .filter(|f| f.extension().map(|e| e == "nes").unwrap_or(false))
                    .next();

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
        let fps = 1.0 / self.last_frame_time.elapsed().as_secs_f64();
        self.last_frame_time = std::time::Instant::now();
        let title = format!(
            "Plastic ({:.0} FPS) {}",
            fps,
            if self.paused { "- Paused" } else { "" }
        );

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    rfd::FileDialog::new()
                        .add_filter("NES ROM", &["nes"])
                        .pick_file()
                        .map(|file| {
                            self.nes = NES::new(file).unwrap();
                        });
                }
                if ui.button("Reset").clicked() {
                    self.nes.reset();
                }
                if ui
                    .button(if self.paused { "Resume" } else { "Pause" })
                    .clicked()
                {
                    self.paused = !self.paused;
                    if !self.paused {
                        // clear the audio buffer
                        _ = self.nes.audio_buffer();
                    }
                }
                if ui.button("Close Game").clicked() {
                    self.nes = NES::new_without_file();
                }
                if ui.button("Exit").clicked() {
                    std::process::exit(0);
                }
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.paused && !self.nes.is_empty() {
            self.nes.clock_for_frame();
            let audio_buffer = self.nes.audio_buffer();
            // convert from 1 channel to 2 channels
            self.audio_player.queue(
                &audio_buffer
                    .iter()
                    .flat_map(|&s| [s, s])
                    .collect::<Vec<_>>(),
            );
            self.audio_player.play().unwrap();
        } else {
            self.audio_player.pause().unwrap();
        }

        self.update_title(ctx);
        self.handle_input(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_menu(ui);
            ui.centered_and_justified(|ui| {
                if !self.nes.is_empty() {
                    {
                        let pixels = self.nes.pixel_buffer();
                        let guard = pixels.lock().unwrap();

                        self.image_texture.set(
                            egui::ColorImage::from_rgba_unmultiplied(
                                [TV_WIDTH, TV_HEIGHT],
                                guard.as_slice(),
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

        ctx.request_repaint();
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
            window_builder: Some(Box::new(|builder| builder.with_drag_and_drop(true))),
            ..Default::default()
        },
        Box::new(|c| Ok(Box::new(App::new(&c.egui_ctx, nes)))),
    )
}
