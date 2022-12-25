mod cpu_debugger;
mod frame;
pub mod step_state;

use crate::cpu::CpuState;
use cpu_debugger::CpuDebugger;
use eframe::egui;
use frame::Frame;
use std::sync::mpsc::{Receiver, Sender};
use step_state::StepState;

#[derive(PartialEq)]
enum View {
    Screen,
    CpuDebugger,
}

pub struct Gui {
    span: tracing::Span,
    frame: Frame,
    cpu_debugger: CpuDebugger,
    current_view: View,
}

impl Gui {
    fn new(
        span: tracing::Span,
        cpu_state_receiver: Receiver<Box<CpuState>>,
        step_sender: Sender<StepState>,
    ) -> Self {
        let cpu_debugger = CpuDebugger::new(cpu_state_receiver, step_sender);
        Self {
            span,
            frame: Frame::new(),
            cpu_debugger,
            current_view: View::Screen,
        }
    }

    pub fn run(
        window_title: &str,
        cpu_state_receiver: Receiver<Box<CpuState>>,
        step_sender: Sender<StepState>,
    ) {
        let span = tracing::span!(tracing::Level::INFO, "gui");
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            window_title,
            options,
            Box::new(|_cc| Box::new(Self::new(span, cpu_state_receiver, step_sender))),
        );
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                ui.label("Open").on_hover_text("TODO");
            });

            ui.menu_button("Show", |ui| {
                ui.radio_value(&mut self.current_view, View::Screen, "Screen");
                ui.radio_value(&mut self.current_view, View::CpuDebugger, "CPU Debugger");
            })
        });
    }
}

impl eframe::App for Gui {
    #[tracing::instrument(skip(self, ctx, _frame), parent = &self.span)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.menu_bar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.current_view {
            View::CpuDebugger => {
                ui.add(self.cpu_debugger.widget());
            }
            View::Screen => {
                ui.add(self.frame.widget());
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

// // To make sure margins are consistent across panels
fn default_frame() -> egui::Frame {
    egui::Frame::central_panel(&egui::Style::default())
}
