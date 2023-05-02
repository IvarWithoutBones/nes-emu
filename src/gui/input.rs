use crate::controller;
use eframe::egui;
use std::sync::mpsc::Sender;

pub struct Input {
    span: tracing::Span,
    pub button_sender: Sender<controller::Buttons>,
}

impl Input {
    pub fn new(button_sender: Sender<controller::Buttons>) -> Self {
        let span = tracing::span!(tracing::Level::INFO, "input");
        Self {
            span,
            button_sender,
        }
    }

    pub fn toggle_pause(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| i.key_pressed(egui::Key::P))
    }

    pub fn step(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| i.key_pressed(egui::Key::O))
    }

    // TODO: This is very, very ugly
    #[tracing::instrument(skip(self, ctx), parent = &self.span)]
    pub fn update(&self, ctx: &egui::Context) {
        let mut button = controller::Buttons::default();

        if ctx.input(|i| i.key_down(egui::Key::Z)) {
            button.set_a(true);
        }

        if ctx.input(|i| i.key_down(egui::Key::X)) {
            button.set_b(true);
        }

        if ctx.input(|i| i.key_down(egui::Key::Space)) {
            button.set_select(true);
        }

        if ctx.input(|i| i.key_down(egui::Key::Enter)) {
            button.set_start(true);
        }

        if ctx.input(|i| i.key_down(egui::Key::ArrowRight)) {
            button.set_right(true);
        }

        if ctx.input(|i| i.key_down(egui::Key::ArrowLeft)) {
            button.set_left(true);
        }

        if ctx.input(|i| i.key_down(egui::Key::ArrowUp)) {
            button.set_up(true);
        }

        if ctx.input(|i| i.key_down(egui::Key::ArrowDown)) {
            button.set_down(true);
        }

        if let Err(e) = self.button_sender.send(button) {
            tracing::error!("failed to send button to controller: {e}, CPU most likely crashed");
            std::process::exit(1);
        }
    }
}
