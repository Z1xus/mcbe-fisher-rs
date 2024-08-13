use eframe::egui;
use egui::{Color32, RichText, Stroke};
use image::ImageReader;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::fisher::Fisher;
use crate::memory::{get_pid, MemoryReader};

pub struct FisherUi {
    fisher: Option<Arc<Fisher>>,
    memory: Option<Arc<MemoryReader>>,
    is_fishing: bool,
    casts: i32,
    custom_casts: i32,
    threshold: u32,
    fishing_thread: Option<thread::JoinHandle<()>>,
    stop_sender: Option<Sender<()>>,
    stop_receiver: Option<Receiver<()>>,
    game_running: bool,
    version: String,
    start_time: Option<Instant>,
    countdown: i32,
}

impl FisherUi {
    pub fn new() -> Self {
        Self {
            fisher: None,
            memory: None,
            is_fishing: false,
            casts: -1,
            custom_casts: 64,
            threshold: 1,
            fishing_thread: None,
            stop_sender: None,
            stop_receiver: None,
            game_running: false,
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: None,
            countdown: 5,
        }
    }

    pub fn run(self) -> Result<(), eframe::Error> {
        let icon_path = format!("{}/resources/icon.png", env!("CARGO_MANIFEST_DIR"));
        let icon_data = load_icon(&icon_path);
        let mut viewport = egui::ViewportBuilder::default()
            .with_inner_size([350.0, 490.0])
            .with_resizable(false);

        if let Some(icon) = icon_data {
            viewport = viewport.with_icon(icon);
        } else {
            eprintln!("failed to load icon");
        }

        let options = eframe::NativeOptions {
            viewport,
            ..Default::default()
        };
        eframe::run_native(
            "mcbe-fisher-rs",
            options,
            Box::new(|_cc| Ok(Box::new(self))),
        )
    }

    fn start_fishing(&mut self) {
        if let Some(pid) = get_pid("Minecraft.Windows.exe") {
            self.game_running = true;
            if self.fisher.is_none() {
                let memory = Arc::new(MemoryReader::new(pid));
                self.memory = Some(memory.clone());
                let fisher = Arc::new(Fisher::new(memory));
                self.fisher = Some(fisher.clone());

                let casts = if self.casts == -1 {
                    None
                } else {
                    Some(self.custom_casts)
                };
                let threshold = self.threshold;

                let (tx, rx) = channel();
                self.stop_sender = Some(tx.clone());
                self.stop_receiver = Some(rx);

                self.start_time = Some(Instant::now());
                self.countdown = 5;

                self.fishing_thread = Some(thread::spawn(move || {
                    fisher.run(casts, threshold, tx);
                }));
            }
            self.is_fishing = true;
        } else {
            self.game_running = false;
            eprintln!("failed to find Minecraft process");
        }
    }

    fn stop_fishing(&mut self) {
        if let Some(fisher) = &self.fisher {
            fisher.stop();
        }

        if let Some(handle) = self.fishing_thread.take() {
            thread::sleep(Duration::from_millis(100));

            for _ in 0..10 {
                if handle.is_finished() {
                    if let Err(e) = handle.join() {
                        eprintln!("error joining fishing thread: {:?}", e);
                    }
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }

        self.is_fishing = false;
        self.fisher = None;
        self.memory = None;
        self.stop_sender = None;
        self.stop_receiver = None;
    }

    fn check_fishing_status(&mut self) {
        if let Some(receiver) = &self.stop_receiver {
            if receiver.try_recv().is_ok() {
                self.stop_fishing();
            }
        }
    }

    fn check_game_status(&mut self) {
        self.game_running = get_pid("Minecraft.Windows.exe").is_some();
    }

    fn create_dark_visuals(&self) -> egui::Visuals {
        egui::Visuals {
            dark_mode: true,
            panel_fill: Color32::from_rgb(13, 17, 23),
            window_fill: Color32::from_rgb(22, 27, 34),
            faint_bg_color: Color32::from_rgb(22, 27, 34),
            window_stroke: Stroke::new(1.0, Color32::from_rgb(48, 54, 61)),
            widgets: egui::style::Widgets::dark(),
            selection: egui::style::Selection::default(),
            ..Default::default()
        }
    }
}

impl eframe::App for FisherUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_fishing_status();
        self.check_game_status();

        ctx.set_visuals(self.create_dark_visuals());

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(25.0);
                ui.heading(RichText::new("mcbe-fisher-rs").size(28.0));
                ui.add_space(15.0);
                ui.label(RichText::new("made by z1xus <3").size(16.0));
                ui.label(RichText::new("tested on mcbe v1.21.2 build 25836796").size(14.0));
                ui.add_space(25.0);

                let button_text = if !self.game_running {
                    RichText::new("Start your game").size(18.0)
                } else if self.is_fishing {
                    if let Some(start_time) = self.start_time {
                        let elapsed = start_time.elapsed().as_secs() as i32;
                        if elapsed < 5 {
                            self.countdown = 5 - elapsed;
                            RichText::new(format!("Starting in {}s...", self.countdown)).size(18.0)
                        } else {
                            self.start_time = None;
                            RichText::new("Stop Fishing").size(18.0)
                        }
                    } else {
                        RichText::new("Stop Fishing").size(18.0)
                    }
                } else {
                    RichText::new("Start Fishing").size(18.0)
                };

                if ui
                    .add_sized([200.0, 40.0], egui::Button::new(button_text))
                    .clicked()
                {
                    if self.game_running {
                        if self.is_fishing {
                            self.stop_fishing();
                        } else {
                            self.start_fishing();
                        }
                    }
                }

                ui.add_space(25.0);

                ui.group(|ui| {
                    ui.set_width(300.0);
                    ui.horizontal(|ui| {
                        ui.add_space(15.0);
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Casts:").size(16.0));
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                ui.radio_value(&mut self.casts, -1, RichText::new("Infinite").size(14.0));
                                ui.radio_value(&mut self.casts, 0, RichText::new("Custom").size(14.0));
                            });

                            if self.casts == 0 {
                                ui.add_space(5.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(10.0);
                                    ui.add(
                                        egui::Slider::new(&mut self.custom_casts, 1..=512)
                                            .text(RichText::new("asts").size(14.0)),
                                    );
                                });
                            }
                            ui.add_space(15.0);
                        });
                    });
                });

                ui.add_space(20.0);

                ui.group(|ui| {
                    ui.set_width(300.0);
                    ui.horizontal(|ui| {
                        ui.add_space(15.0);
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("Threshold:").size(16.0));
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                ui.add(egui::Slider::new(&mut self.threshold, 0..=10));
                            });
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                ui.label(RichText::new("The delay before reeling the rod in").size(12.0));
                            });
                            ui.add_space(15.0);
                        });
                    });
                });

                ui.add_space(25.0);

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new(format!("version {}", self.version)).size(12.0));
                });
            });
        });

        ctx.request_repaint();
    }
}

fn load_icon(path: &str) -> Option<egui::IconData> {
    ImageReader::open(path)
        .ok()
        .and_then(|reader| reader.decode().ok())
        .map(|image| {
            let image = image.into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            egui::IconData {
                rgba,
                width,
                height,
            }
        })
}