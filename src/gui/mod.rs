pub mod cpu_debugger;
mod input;
mod screen;

use self::input::Input;
use crate::{
    controller,
    cpu::{CpuState, StepState},
    ppu::renderer::PixelBuffer,
};
use cpu_debugger::CpuDebugger;
use eframe::egui;
use screen::Screen;
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

#[cfg(target_arch = "wasm32")]
use crate::cpu::{self, Cpu};

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

    // Multithreading is not supported in wasm32, so we have to run the CPU from this thread
    #[cfg(target_arch = "wasm32")]
    cpu: Cpu,
}

impl Gui {
    fn new(
        span: tracing::Span,
        cpu_state_receiver: Receiver<Box<CpuState>>,
        step_sender: Sender<StepState>,
        button_sender: Sender<controller::Buttons>,
        pixel_receiver: Receiver<Box<PixelBuffer>>,
        rom_sender: Sender<PathBuf>,

        #[cfg(target_arch = "wasm32")] bus: crate::bus::Bus,
    ) -> Self {
        Self {
            span,
            rom_sender,
            screen: Screen::new(pixel_receiver),
            cpu_debugger: CpuDebugger::new(cpu_state_receiver, step_sender),
            current_view: View::Screen,
            input: Input::new(button_sender),

            #[cfg(target_arch = "wasm32")]
            cpu: cpu::Cpu::new(bus),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(
        window_title: &str,
        cpu_state_receiver: Receiver<Box<CpuState>>,
        step_sender: Sender<StepState>,
        rom_sender: Sender<PathBuf>,
        button_sender: Sender<controller::Buttons>,
        pixel_receiver: Receiver<Box<PixelBuffer>>,
    ) {
        let span = tracing::span!(tracing::Level::INFO, "gui");
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            window_title,
            options,
            Box::new(|_cc| {
                Box::new(Self::new(
                    span,
                    cpu_state_receiver,
                    step_sender,
                    button_sender,
                    pixel_receiver,
                    rom_sender,
                ))
            }),
        );
    }

    #[cfg(target_arch = "wasm32")]
    pub fn run(
        _window_title: &str,
        cpu_state_receiver: Receiver<Box<CpuState>>,
        step_sender: Sender<StepState>,
        rom_sender: Sender<PathBuf>,
        button_sender: Sender<controller::Buttons>,
        pixel_receiver: Receiver<Box<PixelBuffer>>,
        bus: crate::bus::Bus,
    ) {
        let span = tracing::span!(tracing::Level::INFO, "gui");
        let options = eframe::WebOptions::default();
        wasm_bindgen_futures::spawn_local(async {
            eframe::start_web(
                "egui_canvas_id",
                options,
                Box::new(|_cc| {
                    Box::new(Self::new(
                        span,
                        cpu_state_receiver,
                        step_sender,
                        button_sender,
                        pixel_receiver,
                        rom_sender,
                        bus,
                    ))
                }),
            )
            .await
            .expect("failed to start eframe");
        });
    }

    fn send_rom_path(&mut self, path: PathBuf) {
        tracing::info!("opening ROM file: {}", path.display());
        self.rom_sender.send(path).unwrap_or_else(|err| {
            tracing::error!("failed to send ROM path: {}", err);
        });
    }

    fn update_dropped_files(&mut self, ctx: &egui::Context) {
        for file in &ctx.input().raw.dropped_files.iter().last() {
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

            #[cfg(target_arch = "wasm32")]
            {
                use crate::cartridge::Cartridge;
                tracing::info!("opening ROM file: {}", file.name);
                let cart = Cartridge::from_bytes(&*file.bytes.as_ref().unwrap()).unwrap();
                self.cpu.bus.load_cartridge(cart);
            }
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                let open_file = ui.button("Open").on_hover_text("Open a ROM file");
                if open_file.clicked() {
                    // TODO: use the async file dialog for WASI targets
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("NES ROM", &["nes"])
                        .pick_file()
                    {
                        self.send_rom_path(file);
                    }
                }

                let quit = ui.button("Quit").on_hover_text("Exit the application");
                if quit.clicked() {
                    tracing::info!("quit button clicked, exiting");
                    std::process::exit(0);
                };
            });

            ui.menu_button("Show", |ui| {
                let screen = ui.radio_value(&mut self.current_view, View::Screen, "Screen");
                if screen.clicked() {
                    tracing::info!("switching view to screen");
                }

                let cpu_debugger =
                    ui.radio_value(&mut self.current_view, View::CpuDebugger, "CPU Debugger");
                if cpu_debugger.clicked() {
                    tracing::info!("switching view to CPU debugger");
                }
            })
        });
    }
}

impl eframe::App for Gui {
    #[tracing::instrument(skip(self, ctx, _frame), parent = &self.span)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        {
            if self.cpu.bus.has_cartridge() {
                for i in 0..20 {
                    self.cpu.step();
                }
            }
        }

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
