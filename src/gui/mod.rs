use crate::cheat::{Cheat, CheatRequest};

mod cpu_debugger;
mod input;
mod screen;

use {
    self::{cpu_debugger::CpuDebugger, input::Input, screen::Screen},
    crate::{
        controller,
        cpu::CpuState,
        glue::StepState,
        ppu::renderer::{PixelBuffer, HEIGHT, WIDTH},
        LogReloadHandle,
    },
    eframe::egui,
    std::{
        path::PathBuf,
        sync::mpsc::{Receiver, Sender},
    },
    tracing::metadata::LevelFilter,
    tracing_subscriber::EnvFilter,
};

#[derive(PartialEq)]
enum View {
    Screen,
    CpuDebugger,
}

pub struct Gui {
    span: tracing::Span,
    screen: Screen,
    cpu_debugger: CpuDebugger,
    current_view: View,
    input: Input,

    rom_sender: Sender<PathBuf>,
    unload_rom_sender: Sender<()>,
    reboot_sender: Sender<()>,

    log_reload_handle: LogReloadHandle,
    log_level: LevelFilter,

    cheat_sender: Sender<CheatRequest>,
    cheats: Vec<(bool, Cheat)>,
}

impl Gui {
    #[allow(clippy::too_many_arguments)] // TODO: fix this
    pub fn run(
        window_title: &str,
        log_reload_handle: LogReloadHandle,
        cpu_state_receiver: Receiver<CpuState>,
        button_sender: Sender<controller::Buttons>,
        pixel_receiver: Receiver<Box<PixelBuffer>>,
        cheat_sender: Sender<CheatRequest>,
        (step_sender, reboot_sender): (Sender<StepState>, Sender<()>),
        (rom_sender, unload_rom_sender): (Sender<PathBuf>, Sender<()>),
    ) {
        let span = tracing::span!(tracing::Level::INFO, "gui");
        let log_level = log_reload_handle
            .with_current(|current| current.max_level_hint())
            .unwrap_or(Some(LevelFilter::ERROR))
            .unwrap();

        const INITIAL_SCALE: f32 = 3.0;
        let options = eframe::NativeOptions {
            initial_window_size: Some(egui::Vec2::new(
                WIDTH as f32 * INITIAL_SCALE,
                HEIGHT as f32 * INITIAL_SCALE,
            )),
            ..Default::default()
        };

        let gui = Self {
            span,
            rom_sender,
            reboot_sender,
            unload_rom_sender,
            screen: Screen::new(pixel_receiver),
            cpu_debugger: CpuDebugger::new(cpu_state_receiver, step_sender),
            current_view: View::Screen,
            input: Input::new(button_sender),

            cheat_sender,
            cheats: Vec::new(),

            log_reload_handle,
            log_level,
        };

        eframe::run_native(window_title, options, Box::new(|_cc| Box::new(gui))).unwrap_or_else(
            |err| {
                tracing::error!("failed to run GUI: {}", err);
            },
        );
    }

    fn load_cheat_file(&mut self, path: PathBuf) {
        tracing::info!("loading cheats from file: {}", path.display());
        let maybe_file = std::fs::read_to_string(path);

        if let Ok(file) = maybe_file {
            self.cheats = Cheat::from_multiple(&file)
                .filter_map(|cheat| {
                    if cheat.ty.is_compare() {
                        None // Only ReadSubstitute is currently supported
                    } else {
                        Some((false, cheat))
                    }
                })
                .collect();

            tracing::info!("loaded {} cheats", self.cheats.len());
        } else {
            tracing::error!("failed to load cheat file: {}", maybe_file.unwrap_err());
        }
    }

    fn send_rom_path(&mut self, path: PathBuf) {
        self.unload_rom(); // In case one is already loaded, does nothing otherwise
        tracing::info!("opening ROM file: {}", path.display());
        self.rom_sender.send(path).unwrap_or_else(|err| {
            tracing::error!("failed to send ROM path: {}", err);
        });
    }

    fn unload_rom(&mut self) {
        tracing::info!("closing ROM");
        self.unload_rom_sender.send(()).unwrap_or_else(|err| {
            tracing::error!("failed to send unload ROM signal: {err}");
        });
        self.cpu_debugger.update_buffer();
        self.cpu_debugger.clear_states();
    }

    fn update_dropped_files(&mut self, ctx: &egui::Context) {
        for file in &ctx.input(|i| i.raw.dropped_files.iter().last().cloned()) {
            if let Some(path) = &file.path {
                if path.extension().unwrap_or_default() == "nes" {
                    self.send_rom_path(path.to_path_buf());
                } else {
                    tracing::warn!(
                        "dropped file '{}' does not have a .nes file extension! ignoring",
                        path.display()
                    );
                }
            }
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                let open_file = ui.button("Open").on_hover_text("Open a ROM file");
                if open_file.clicked() {
                    ui.close_menu();
                    self.cpu_debugger.pause();

                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("NES ROM", &["nes"])
                        .pick_file()
                    {
                        self.send_rom_path(file);
                    }

                    self.cpu_debugger.unpause();
                }

                let close_file = ui.button("Close").on_hover_text("Close the current ROM");
                if close_file.clicked() {
                    ui.close_menu();
                    self.unload_rom();
                }

                let quit = ui.button("Quit").on_hover_text("Exit the application");
                if quit.clicked() {
                    ui.close_menu();
                    tracing::info!("quit button clicked, exiting");
                    std::process::exit(0);
                };
            });

            ui.menu_button("Emulation", |ui| {
                let reboot = ui.button("Reboot").on_hover_text("Reboot the emulator");
                if reboot.clicked() {
                    ui.close_menu();
                    tracing::info!("reboot button clicked, restarting emulator");
                    self.reboot_sender.send(()).unwrap_or_else(|err| {
                        tracing::error!("failed to send reboot signal: {}", err);
                    });
                    self.cpu_debugger.unpause();
                }

                let pause = ui
                    .button("Toggle Pause")
                    .on_hover_text("Pause or unpause the emulator");
                if pause.clicked() {
                    ui.close_menu();
                    self.cpu_debugger.toggle_pause();
                }

                let step = ui
                    .button("Step")
                    .on_hover_text("Step the emulator one instruction");
                if step.clicked() {
                    ui.close_menu();
                    self.cpu_debugger.step();
                }
            });

            ui.menu_button("Show", |ui| {
                let screen = ui.radio_value(&mut self.current_view, View::Screen, "Screen");
                if screen.clicked() {
                    tracing::info!("switching view to screen");
                    ui.close_menu();
                }

                let cpu_debugger =
                    ui.radio_value(&mut self.current_view, View::CpuDebugger, "CPU Debugger");
                if cpu_debugger.clicked() {
                    tracing::info!("switching view to CPU debugger");
                    ui.close_menu();
                }
            });

            ui.menu_button("Cheats", |ui| {
                if ui.button("Load").clicked() {
                    self.cpu_debugger.pause();
                    ui.close_menu();

                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("Cheat File", &["cht"])
                        .pick_file()
                    {
                        self.load_cheat_file(file);
                    }

                    self.cpu_debugger.unpause();
                }

                if ui.button("Clear").clicked() {
                    tracing::info!("clearing cheats");
                    ui.close_menu();
                    self.cheats.clear();

                    self.cheat_sender
                        .send(CheatRequest::Clear)
                        .unwrap_or_else(|err| {
                            tracing::error!("failed to send clear cheat request: {err}");
                        });
                }

                if !self.cheats.is_empty() {
                    ui.separator();

                    for (index, (enabled, cheat)) in &mut self.cheats.iter_mut().enumerate() {
                        let name = cheat
                            .description
                            .clone()
                            .unwrap_or_else(|| format!("Cheat #{index}"));

                        if ui.checkbox(enabled, name.clone()).clicked() {
                            ui.close_menu();

                            if *enabled {
                                tracing::info!("adding cheat: {name}");
                                self.cheat_sender
                                    .send(CheatRequest::Add(cheat.clone()))
                                    .unwrap_or_else(|err| {
                                        tracing::error!("failed to send add cheat request: {err}");
                                    });
                            } else {
                                tracing::info!("removing cheat: {name}");
                                self.cheat_sender
                                    .send(CheatRequest::Remove(cheat.clone()))
                                    .unwrap_or_else(|err| {
                                        tracing::error!(
                                            "failed to send remove cheat request: {err}"
                                        );
                                    });
                            }
                        }
                    }
                }
            });

            ui.menu_button("Log", |ui| {
                self.log_level_button(ui, LevelFilter::ERROR);
                self.log_level_button(ui, LevelFilter::WARN);
                self.log_level_button(ui, LevelFilter::INFO);
                self.log_level_button(ui, LevelFilter::DEBUG);
                self.log_level_button(ui, LevelFilter::TRACE);
            })
        });
    }

    fn log_level_button(&mut self, ui: &mut egui::Ui, level: LevelFilter) {
        let button = ui.radio_value(&mut self.log_level, level, level.to_string());
        if button.clicked() {
            tracing::info!("switching log level to {level}");
            self.log_reload_handle
                .modify(|filter| *filter = EnvFilter::from(level.to_string()))
                .unwrap_or_else(|err| {
                    tracing::error!("failed to modify log level: {err}");
                });
            ui.close_menu();
        }
    }
}

impl eframe::App for Gui {
    #[tracing::instrument(skip(self, ctx, _frame), parent = &self.span)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Not updating the buffers will cause a memory leak because the MPSC channels wont be emptied.
        // TODO: Switch to a bounded crossbeam channel to avoid this.
        self.input.update(ctx);
        self.screen.update_buffer(ctx);
        self.cpu_debugger.update_buffer();
        self.update_dropped_files(ctx);

        if self.input.toggle_pause(ctx) {
            self.cpu_debugger.toggle_pause();
        }

        if self.input.step(ctx) {
            self.cpu_debugger.step();
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.menu_bar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.current_view {
            View::CpuDebugger => {
                ui.add(self.cpu_debugger.widget());
            }
            View::Screen => {
                ui.add(self.screen.widget());
            }
        });

        // Calling this here will request another frame immediately after this one
        ctx.request_repaint();
    }
}

fn header_label(ui: &mut egui::Ui, name: &str) {
    ui.vertical_centered(|ui| {
        ui.heading(egui::RichText::new(name).strong());
    });
}

/// Make sure margins are consistent across panels
fn default_frame() -> egui::Frame {
    egui::Frame::central_panel(&egui::Style::default())
}
