pub mod cpu_debugger;
mod screen;

use crate::{cpu::CpuState, ppu::renderer::PixelBuffer};
use cpu_debugger::{step_state::StepState, CpuDebugger};
use eframe::egui;
use screen::Screen;
use std::sync::mpsc::{Receiver, Sender};

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
}

impl Gui {
    fn new(
        span: tracing::Span,
        cpu_state_receiver: Receiver<Box<CpuState>>,
        step_sender: Sender<StepState>,
        pixel_receiver: Receiver<Box<PixelBuffer>>,
    ) -> Self {
        Self {
            span,
            screen: Screen::new(pixel_receiver),
            cpu_debugger: CpuDebugger::new(cpu_state_receiver, step_sender),
            current_view: View::Screen,
        }
    }

    pub fn run(
        window_title: &str,
        cpu_state_receiver: Receiver<Box<CpuState>>,
        step_sender: Sender<StepState>,
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
                    pixel_receiver,
                ))
            }),
        );
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                let open_file = ui.button("Open").on_hover_text("TODO");
                if open_file.clicked() {
                    tracing::warn!("Open file is not implemented");
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
        // Not updating the buffers will cause a memory leak because the MPSC channels wont be emptied.
        // TODO: Switch to a bounded crossbeam channel to avoid this.
        self.screen.update_buffer(ctx);
        self.cpu_debugger.update_buffer();

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
